use crate::http_client::HttpClient;
use crate::http_request::HttpRequest;
use gpui::BackgroundExecutor;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

/// Lightweight metric for stress testing - minimal memory footprint
#[derive(Debug, Clone)]
pub struct RequestMetric {
    pub timestamp: f64, // seconds since test start
    pub response_time_ms: f64,
    pub status_code: u16,
    pub success: bool,
    pub response_size: usize,
}

/// Configuration for stress test run
pub struct StressTestConfig {
    pub request: HttpRequest,
    pub requests_per_second: usize,
    pub duration_secs: u64,
    pub background_executor: BackgroundExecutor,
}

/// Handle to control a running stress test
pub struct StressTestHandle {
    pub cancel_token: CancellationToken,
    pub metrics_rx: mpsc::UnboundedReceiver<RequestMetric>,
}

impl StressTestHandle {
    pub fn cancel(&self) {
        self.cancel_token.cancel();
    }

    pub async fn next_metric(&mut self) -> Option<RequestMetric> {
        self.metrics_rx.recv().await
    }

    pub fn split(self) -> (CancellationToken, mpsc::UnboundedReceiver<RequestMetric>) {
        (self.cancel_token, self.metrics_rx)
    }
}

/// Stress test engine - manages concurrent HTTP requests with rate limiting
pub struct StressEngine;

impl StressEngine {
    /// Start a new stress test run
    pub fn start(config: StressTestConfig) -> StressTestHandle {
        let cancel_token = CancellationToken::new();
        let (metrics_tx, metrics_rx) = mpsc::unbounded_channel();

        let cancel_clone = cancel_token.clone();
        let executor = config.background_executor.clone();

        executor
            .spawn(async move {
                Self::run_test(config, cancel_clone, metrics_tx).await;
            })
            .detach();

        StressTestHandle {
            cancel_token,
            metrics_rx,
        }
    }

    /// Main test loop - rate-limited request spawning
    async fn run_test(
        config: StressTestConfig,
        cancel_token: CancellationToken,
        metrics_tx: mpsc::UnboundedSender<RequestMetric>,
    ) {
        let test_start = Instant::now();
        let client = HttpClient::global();
        let executor = config.background_executor.clone();
        let request = Arc::new(config.request);

        // Calculate interval between requests for target RPS
        let interval_ms = 1000 / config.requests_per_second.max(1);
        let mut interval =
            tokio::time::interval(std::time::Duration::from_millis(interval_ms as u64));

        // Skip missed ticks to avoid bursts after slow periods
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            // Check stop conditions
            if cancel_token.is_cancelled() {
                break;
            }

            if test_start.elapsed().as_secs() >= config.duration_secs {
                break;
            }

            interval.tick().await;

            // Spawn individual request task (non-blocking)
            let request_clone = request.clone();
            let metrics_tx_clone = metrics_tx.clone();
            let elapsed = test_start.elapsed();

            executor
                .spawn(async move {
                    let metric = Self::execute_request(client, &request_clone, elapsed).await;
                    let _ = metrics_tx_clone.send(metric);
                })
                .detach();
        }
    }

    /// Execute a single request and capture metrics
    async fn execute_request(
        client: &HttpClient,
        request: &HttpRequest,
        test_elapsed: std::time::Duration,
    ) -> RequestMetric {
        let timestamp = test_elapsed.as_secs_f64();

        match client.execute_lean(request).await {
            Ok((status, latency_ms, size)) => RequestMetric {
                timestamp,
                response_time_ms: latency_ms,
                status_code: status,
                success: (200..300).contains(&status),
                response_size: size,
            },
            Err(_) => RequestMetric {
                timestamp,
                response_time_ms: 0.0,
                status_code: 0,
                success: false,
                response_size: 0,
            },
        }
    }
}

/// Aggregated statistics for a stress test
#[derive(Default, Clone)]
pub struct StressTestStats {
    pub total_requests: usize,
    pub successful_requests: usize,
    pub failed_requests: usize,
    pub total_bytes: usize,
    pub min_latency_ms: f64,
    pub max_latency_ms: f64,
    pub avg_latency_ms: f64,
}

impl StressTestStats {
    pub fn new() -> Self {
        Self {
            min_latency_ms: f64::MAX,
            max_latency_ms: 0.0,
            ..Default::default()
        }
    }

    pub fn update(&mut self, metric: &RequestMetric) {
        self.total_requests += 1;
        self.total_bytes += metric.response_size;

        if metric.success {
            self.successful_requests += 1;

            // Update latency stats (only for successful requests)
            if metric.response_time_ms > 0.0 {
                self.min_latency_ms = self.min_latency_ms.min(metric.response_time_ms);
                self.max_latency_ms = self.max_latency_ms.max(metric.response_time_ms);

                // Running average
                let prev_total = (self.successful_requests - 1) as f64 * self.avg_latency_ms;
                self.avg_latency_ms =
                    (prev_total + metric.response_time_ms) / self.successful_requests as f64;
            }
        } else {
            self.failed_requests += 1;
        }
    }

    pub fn success_rate(&self) -> f64 {
        if self.total_requests > 0 {
            (self.successful_requests as f64 / self.total_requests as f64) * 100.0
        } else {
            0.0
        }
    }
}
