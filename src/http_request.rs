use crate::auth::AuthType;
use crate::fs::FileContent;
use crate::http_response::AuthPayload;

#[derive(Clone)]
pub struct HttpRequest {
    pub url: String,
    pub method: String,
    pub query_params: Vec<(String, String)>,
    pub headers: Vec<(String, String)>,
    pub body: String,
    pub auth: AuthPayload,
}
impl HttpRequest {
    pub fn new(method: &str, url: &str) -> Self {
        Self {
            url: url.to_string(),
            method: method.to_string(),
            query_params: vec![],
            headers: vec![],
            body: String::new(),
            auth: AuthPayload::None,
        }
    }

    pub fn from_file_content(content: &FileContent) -> Self {
        let auth = match &content.auth.auth_type {
            AuthType::Basic => {
                if !content.auth.username.is_empty() && !content.auth.password.is_empty() {
                    AuthPayload::Basic {
                        username: content.auth.username.clone(),
                        password: content.auth.password.clone(),
                    }
                } else {
                    AuthPayload::None
                }
            }
            AuthType::Bearer => {
                if !content.auth.token.is_empty() {
                    AuthPayload::Bearer {
                        token: content.auth.token.clone(),
                    }
                } else {
                    AuthPayload::None
                }
            }
            AuthType::None => AuthPayload::None,
        };

        // Convert KeyValue to (String, String) and filter active only
        let query_params: Vec<(String, String)> = content
            .params
            .iter()
            .filter(|kv| kv.active)
            .map(|kv| (kv.key.clone(), kv.value.clone()))
            .collect();

        let headers: Vec<(String, String)> = content
            .headers
            .iter()
            .filter(|kv| kv.active)
            .map(|kv| (kv.key.clone(), kv.value.clone()))
            .collect();

        Self {
            url: content.url.clone(),
            method: content.method.clone(),
            query_params,
            headers,
            body: content.body.body.clone(),
            auth,
        }
    }
    pub fn method(mut self, method: &str) -> Self {
        self.method = method.to_string();
        self
    }
    pub fn url(mut self, url: &str) -> Self {
        self.url = url.to_string();
        self
    }
    pub fn header(mut self, key: &str, value: &str) -> Self {
        self.headers.push((key.to_string(), value.to_string()));
        self
    }
    pub fn headers(mut self, headers: Vec<(String, String)>) -> Self {
        self.headers = headers;
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
}
