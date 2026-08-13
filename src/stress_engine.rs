use crate::http_client::HttpClient;
use crate::http_request::HttpRequest;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

const REQUEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

#[derive(Debug, Clone)]
pub struct RequestMetric {
    pub timestamp: f64,
    pub response_time_ms: f64,
    pub status_code: u16,
    pub success: bool,
    pub response_size: usize,
}

pub struct StressTestConfig {
    pub request: HttpRequest,
    pub requests_per_second: usize,
    pub duration_secs: u64,
}

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

pub struct StressEngine;

impl StressEngine {
    pub fn start(config: StressTestConfig) -> StressTestHandle {
        let cancel_token = CancellationToken::new();
        let (metrics_tx, metrics_rx) = mpsc::unbounded_channel();

        let cancel_clone = cancel_token.clone();

        HttpClient::runtime().spawn(async move {
            Self::run_test(config, cancel_clone, metrics_tx).await;
        });

        StressTestHandle {
            cancel_token,
            metrics_rx,
        }
    }

    async fn run_test(
        config: StressTestConfig,
        cancel_token: CancellationToken,
        metrics_tx: mpsc::UnboundedSender<RequestMetric>,
    ) {
        let test_start = Instant::now();
        let client = HttpClient::global();
        let request = Arc::new(config.request);

        let interval_ms = 1000 / config.requests_per_second.max(1);
        let mut interval =
            tokio::time::interval(std::time::Duration::from_millis(interval_ms as u64));

        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        let total_requests = config.requests_per_second.saturating_mul(config.duration_secs as usize);
        let mut tasks = tokio::task::JoinSet::new();

        for _ in 0..total_requests {
            if cancel_token.is_cancelled() {
                break;
            }

            interval.tick().await;

            let request_clone = request.clone();
            let metrics_tx_clone = metrics_tx.clone();
            let elapsed = test_start.elapsed();

            tasks.spawn(async move {
                let metric = Self::execute_request(client, &request_clone, elapsed).await;
                let _ = metrics_tx_clone.send(metric);
            });
        }

        while tasks.join_next().await.is_some() {
            if cancel_token.is_cancelled() {
                tasks.abort_all();
                break;
            }
        }
    }

    async fn execute_request(
        client: &HttpClient,
        request: &HttpRequest,
        test_elapsed: std::time::Duration,
    ) -> RequestMetric {
        let timestamp = test_elapsed.as_secs_f64();

        match tokio::time::timeout(REQUEST_TIMEOUT, client.execute_lean(request)).await {
            Ok(Ok((status, latency_ms, size))) => RequestMetric {
                timestamp,
                response_time_ms: latency_ms,
                status_code: status,
                success: (200..300).contains(&status),
                response_size: size,
            },
            Ok(Err(_)) => RequestMetric {
                timestamp,
                response_time_ms: 0.0,
                status_code: 0,
                success: false,
                response_size: 0,
            },
            Err(_) => RequestMetric {
                timestamp,
                response_time_ms: REQUEST_TIMEOUT.as_secs_f64() * 1000.0,
                status_code: 0,
                success: false,
                response_size: 0,
            },
        }
    }
}

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

            if metric.response_time_ms > 0.0 {
                self.min_latency_ms = self.min_latency_ms.min(metric.response_time_ms);
                self.max_latency_ms = self.max_latency_ms.max(metric.response_time_ms);

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
