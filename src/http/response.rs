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
