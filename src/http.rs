use std::sync::OnceLock;

use reqwest::header::{HeaderMap, HeaderName, HeaderValue};

static RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

pub fn runtime() -> &'static tokio::runtime::Runtime {
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .expect("failed to build tokio runtime")
    })
}

pub fn client() -> &'static reqwest::Client {
    CLIENT.get_or_init(|| reqwest::Client::new())
}

#[derive(Clone)]
pub enum AuthPayload {
    None,
    Basic { username: String, password: String },
    Bearer { token: String },
}

#[derive(Clone)]
pub struct ResponseHeaders {
    pub headers: Vec<(String, String)>,
    pub response_size: usize,
}

#[derive(Clone)]
pub struct ResponseBody {
    pub body: String,
    pub response_size: usize,
}

#[derive(Clone)]
pub struct RequestStats {
    pub header_size: usize,
    pub body_size: usize,
    pub size: usize,
}

#[derive(Clone)]
pub struct Response {
    pub status_code: u16,
    pub status_text: String,
    pub headers: ResponseHeaders,
    pub body: ResponseBody,
    pub cookies: Vec<(String, String)>,
    pub request: RequestStats,
    pub response_size: usize,
    pub duration: std::time::Duration,
}

#[derive(Clone)]
pub struct HttpRequest {
    url: String,
    method: String,
    query_params: Vec<(String, String)>,
    headers: HeaderMap,
    raw_headers: Vec<(String, String)>,
    body: String,
    auth: AuthPayload,
}

impl HttpRequest {
    pub fn new() -> Self {
        Self {
            url: String::new(),
            method: "GET".to_string(),
            query_params: vec![],
            headers: HeaderMap::new(),
            raw_headers: vec![],
            body: String::new(),
            auth: AuthPayload::None,
        }
    }

    pub fn url(mut self, url: &str) -> Self {
        self.url = url.to_string();
        self
    }

    pub fn method(mut self, method: &str) -> Self {
        self.method = method.to_string();
        self
    }

    pub fn header(mut self, key: &str, value: &str) -> Self {
        if let (Ok(name), Ok(val)) = (
            HeaderName::from_bytes(key.as_bytes()),
            HeaderValue::from_str(value),
        ) {
            self.headers.insert(name, val);
        }
        self.raw_headers.push((key.to_string(), value.to_string()));
        self
    }

    pub fn headers(mut self, headers: Vec<(String, String)>) -> Self {
        for (key, value) in &headers {
            if let (Ok(name), Ok(val)) = (
                HeaderName::from_bytes(key.as_bytes()),
                HeaderValue::from_str(value),
            ) {
                self.headers.insert(name, val);
            }
        }
        self.raw_headers = headers;
        self
    }

    pub fn query(mut self, key: &str, value: &str) -> Self {
        self.query_params.push((key.to_string(), value.to_string()));
        self
    }

    pub fn queries(mut self, params: Vec<(String, String)>) -> Self {
        self.query_params = params;
        self
    }

    pub fn body(mut self, body: &str) -> Self {
        self.body = body.to_string();
        self
    }

    pub fn auth(mut self, auth: AuthPayload) -> Self {
        self.auth = auth;
        self
    }

    pub async fn send(self) -> anyhow::Result<Response> {
        let http_method = reqwest::Method::from_bytes(self.method.as_bytes())?;
        let (tx, rx) = tokio::sync::oneshot::channel();
        let request_body = self.body.clone();
        let request_headers = self.raw_headers.clone();
        let request_header_size = request_headers
            .iter()
            .map(|(k, v)| k.len() + 2 + v.len() + 2)
            .sum::<usize>();
        let request_body_size = request_body.len();
        let request_size = request_header_size + request_body_size;

        runtime().spawn(async move {
            let result = async {
                let mut req = client().request(http_method, &self.url);

                match &self.auth {
                    AuthPayload::None => {}
                    AuthPayload::Basic { username, password } => {
                        req = req.basic_auth(username, Some(password));
                    }
                    AuthPayload::Bearer { token } => req = req.bearer_auth(token),
                }

                if !self.body.is_empty() {
                    req = req.body(self.body);
                }

                req = req.headers(self.headers);

                if !self.query_params.is_empty() {
                    let query_pairs: Vec<(&str, &str)> = self
                        .query_params
                        .iter()
                        .map(|(k, v)| (k.as_str(), v.as_str()))
                        .collect();
                    req = req.query(&query_pairs);
                }

                let start = std::time::Instant::now();

                let resp = req.send().await?;

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

                Ok::<_, anyhow::Error>(Response {
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
            .await;
            let _ = tx.send(result);
        });

        rx.await?
    }
}
