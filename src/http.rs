// somewhere global, e.g. in http.rs
use std::sync::OnceLock;
static RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();

pub fn runtime() -> &'static tokio::runtime::Runtime {
    RUNTIME.get_or_init(|| tokio::runtime::Runtime::new().expect("failed to build tokio runtime"))
}

pub async fn send_get(url: &str) -> anyhow::Result<String> {
    let url = url.to_string();
    let (tx, rx) = tokio::sync::oneshot::channel();
    runtime().spawn(async move {
        let result = async {
            let resp = reqwest::get(&url).await?;
            let body = resp.text().await?;
            Ok::<_, anyhow::Error>(body)
        }
        .await;
        let _ = tx.send(result);
    });
    rx.await?
}
