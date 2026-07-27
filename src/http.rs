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

pub async fn send_request(
    url: &str,
    method: &str,
    query_params: Vec<(String, String)>,
    headers: Vec<(String, String)>,
    body: String,
) -> anyhow::Result<(String, Vec<(String, String)>)> {
    let url = url.to_string();
    let mut req_headers = HeaderMap::new();
    for (key, value) in &headers {
        if let (Ok(name), Ok(val)) = (
            HeaderName::from_bytes(key.as_bytes()),
            HeaderValue::from_str(value),
        ) {
            req_headers.insert(name, val);
        }
    }
    let http_method = reqwest::Method::from_bytes(method.as_bytes())?;
    let (tx, rx) = tokio::sync::oneshot::channel();

    runtime().spawn(async move {
        let result = async {
            let mut req = client().request(http_method, &url).headers(req_headers);

            if !body.is_empty() {
                req = req.body(body);
            }

            if !query_params.is_empty() {
                let query_pairs: Vec<(&str, &str)> = query_params
                    .iter()
                    .map(|(k, v)| (k.as_str(), v.as_str()))
                    .collect();
                req = req.query(&query_pairs);
            }
            let resp = req.send().await?;
            let headers: Vec<(String, String)> = resp
                .headers()
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
                .collect();
            let body = resp.text().await?;
            Ok::<_, anyhow::Error>((body, headers))
        }
        .await;
        let _ = tx.send(result);
    });
    rx.await?
}
