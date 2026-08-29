use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use std::sync::OnceLock;
use tokio::sync::oneshot;

use crate::{
    fs::env::interpolate,
    http_request::HttpRequest,
    http_response::{AuthPayload, RequestStats, Response, ResponseBody, ResponseHeaders},
};
static RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
static CLIENT: OnceLock<HttpClient> = OnceLock::new();

pub struct PendingSend {
    abort: tokio::task::AbortHandle,
    rx: oneshot::Receiver<anyhow::Result<Response>>,
}

impl PendingSend {
    pub async fn wait(self) -> anyhow::Result<Response> {
        self.rx
            .await
            .map_err(|_| anyhow::anyhow!("Request cancelled"))?
    }
    pub fn cancel(&self) {
        self.abort.abort();
    }
    pub fn cancel_handle(&self) -> tokio::task::AbortHandle {
        self.abort.clone()
    }
}

#[derive(Clone)]
pub struct HttpClient {
    client: reqwest::Client,
}

impl HttpClient {
    pub fn global() -> &'static HttpClient {
        CLIENT.get_or_init(|| HttpClient {
            client: reqwest::Client::new(),
        })
    }
    pub fn runtime() -> &'static tokio::runtime::Runtime {
        RUNTIME.get_or_init(|| {
            tokio::runtime::Builder::new_multi_thread()
                .worker_threads(4)
                .enable_all()
                .build()
                .expect("failed to build tokio runtime")
        })
    }

    pub fn request(&self, method: &str, url: &str) -> HttpRequest {
        HttpRequest::new(method, url)
    }

    fn build_request(&self, request: &HttpRequest) -> anyhow::Result<reqwest::Request> {
        let http_method = reqwest::Method::from_bytes(request.method.as_bytes())?;
        let url = interpolate(&request.url);
        let mut req = self.client.request(http_method, &url);
        match &request.auth {
            AuthPayload::None => {}
            AuthPayload::Basic { username, password } => {
                req = req.basic_auth(interpolate(username), Some(interpolate(password)));
            }
            AuthPayload::Bearer { token } => req = req.bearer_auth(interpolate(token)),
        }
        if !request.body.is_empty() {
            req = req.body(interpolate(&request.body.clone()));
        }
        let mut headers = HeaderMap::new();
        for (key, value) in &request.headers {
            let interpolated = interpolate(value);
            if let (Ok(name), Ok(val)) = (
                HeaderName::from_bytes(key.as_bytes()),
                HeaderValue::from_str(&interpolated),
            ) {
                headers.insert(name, val);
            }
        }
        req = req.headers(headers);
        if !request.query_params.is_empty() {
            let params: Vec<(String, String)> = request
                .query_params
                .iter()
                .map(|(k, v)| (k.clone(), interpolate(v)))
                .collect();
            req = req.query(&params);
        }
        Ok(req.build()?)
    }

    pub async fn execute_async(&self, request: &HttpRequest) -> anyhow::Result<Response> {
        let req = self.build_request(request)?;
        let request_header_size = request
            .headers
            .iter()
            .map(|(k, v)| k.len() + 2 + v.len() + 2)
            .sum::<usize>();
        let request_body_size = request.body.len();
        let request_size = request_header_size + request_body_size;
        let start = std::time::Instant::now();
        let resp = self.client.execute(req).await?;
        let status_code = resp.status().as_u16();
        let status_text = resp
            .status()
            .canonical_reason()
            .unwrap_or("Unknown")
            .to_string();
        let resp_headers = resp.headers().clone();
        let response_headers: Vec<(String, String)> = resp_headers
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();
        let cookies = resp
            .cookies()
            .map(|cookie| (cookie.name().to_string(), cookie.value().to_string()))
            .collect::<Vec<_>>();
        let body_bytes = resp.bytes().await?;
        let body = String::from_utf8_lossy(&body_bytes).into_owned();
        let elapsed = start.elapsed();
        let response_body_size = body_bytes.len();
        let header_bytes = resp_headers
            .iter()
            .map(|(k, v)| k.as_str().len() + 2 + v.as_bytes().len() + 2)
            .sum::<usize>();
        let status_line = format!("HTTP/1.1 {} {}\r\n", status_code, status_text);
        let response_header_size = status_line.len() + header_bytes + 2;
        let response_size = response_header_size + response_body_size;
        Ok(Response {
            status_code,
            status_text,
            headers: ResponseHeaders {
                headers: response_headers,
                response_size: response_header_size,
            },
            body: ResponseBody {
                body,
                response_size: response_body_size,
            },
            cookies,
            request: RequestStats {
                header_size: request_header_size,
                body_size: request_body_size,
                size: request_size,
            },
            response_size,
            duration: elapsed,
        })
    }

    pub async fn execute_lean(&self, request: &HttpRequest) -> anyhow::Result<(u16, f64, usize)> {
        let req = self.build_request(request)?;
        let start = std::time::Instant::now();
        let resp = self.client.execute(req).await?;
        let latency_ms = start.elapsed().as_secs_f64() * 1000.0;
        let status = resp.status().as_u16();
        let bytes = resp.bytes().await?.len();
        Ok((status, latency_ms, bytes))
    }

    pub fn send(&self, request: &HttpRequest) -> PendingSend {
        let client = self.client.clone();
        let request = request.clone();
        let (tx, rx) = tokio::sync::oneshot::channel();
        let handle = Self::runtime().spawn(async move {
            let http = HttpClient { client };
            let result = http.execute_async(&request).await;
            let _ = tx.send(result);
        });
        PendingSend {
            abort: handle.abort_handle(),
            rx,
        }
    }
}
