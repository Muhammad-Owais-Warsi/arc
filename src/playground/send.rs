use gpui_kit::*;

use crate::auth::AuthType;
use crate::http::HttpClient;
use crate::http::{AuthPayload, RequestStats, Response, ResponseBody, ResponseHeaders};

use super::model::{RequestPlayground, RequestPlaygroundEvent};

impl RequestPlayground {
    pub fn send_request(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let content = self.current_content(cx);
        let query_params: Vec<(String, String)> = content
            .params
            .into_iter()
            .filter(|kv| kv.active)
            .map(|kv| (kv.key, kv.value))
            .collect();
        let headers: Vec<(String, String)> = content
            .headers
            .into_iter()
            .filter(|kv| kv.active)
            .map(|kv| (kv.key, kv.value))
            .collect();

        let auth = match content.auth.auth_type {
            AuthType::None => AuthPayload::None,
            AuthType::Basic => AuthPayload::Basic {
                username: content.auth.username,
                password: content.auth.password,
            },
            AuthType::Bearer => AuthPayload::Bearer {
                token: content.auth.token,
            },
        };

        let response_panel = self.response_panel();

        response_panel.update(cx, |panel, cx| panel.open(cx));
        cx.emit(RequestPlaygroundEvent::ResponsePanelOpened);

        let request = HttpClient::global()
            .request(&content.method, &content.url)
            .headers(headers)
            .queries(query_params)
            .body(&content.body.body)
            .auth(auth);
        let pending = HttpClient::global().send(&request);
        self.set_pending(pending.cancel_handle());

        let rp = response_panel;
        cx.spawn(async move |this, cx| {
            let result = pending.wait().await;
            let _ = this.update_in(cx, |_this, window, cx| {
                match result {
                    Ok(response) => {
                        rp.update(cx, |p, cx| {
                            p.set_response(response, window, cx);
                        });
                    }
                    Err(err) => {
                        rp.update(cx, |p, cx| {
                            p.set_response(
                                Response {
                                    status_code: 0,
                                    status_text: "Error".to_string(),
                                    headers: ResponseHeaders {
                                        headers: vec![],
                                        response_size: 0,
                                    },
                                    body: ResponseBody {
                                        body: format!("Error: {err}"),
                                        response_size: 0,
                                    },
                                    cookies: vec![],
                                    request: RequestStats {
                                        header_size: 0,
                                        body_size: 0,
                                        size: 0,
                                    },
                                    response_size: 0,
                                    duration: std::time::Duration::ZERO,
                                },
                                window,
                                cx,
                            );
                        });
                    }
                }
                _this.clear_pending();
                cx.notify();
            });
        })
        .detach();
    }
}

impl Drop for RequestPlayground {
    fn drop(&mut self) {
        self.cancel_pending();
    }
}
