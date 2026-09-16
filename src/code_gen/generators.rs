use crate::http_request::HttpRequest;
use crate::http_response::AuthPayload;

use super::model::CodeScreen;

impl CodeScreen {
    pub fn generate_all(req: &HttpRequest) -> Vec<String> {
        vec![
            Self::to_curl(req),
            Self::to_fetch(req),
            Self::to_axios(req),
            Self::to_python(req),
            Self::to_rust(req),
        ]
    }

    fn url_encode(s: &str) -> String {
        let mut out = String::with_capacity(s.len());
        for b in s.bytes() {
            match b {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                    out.push(b as char)
                }
                b' ' => out.push_str("%20"),
                _ => out.push_str(&format!("%{b:02X}")),
            }
        }
        out
    }

    fn full_url(req: &HttpRequest) -> String {
        let base = if req.url.is_empty() {
            "http://localhost".to_string()
        } else {
            req.url.clone()
        };
        if req.query_params.is_empty() {
            return base;
        }
        let sep = if base.contains('?') { '&' } else { '?' };
        let qs = req
            .query_params
            .iter()
            .map(|(k, v)| format!("{}={}", Self::url_encode(k), Self::url_encode(v)))
            .collect::<Vec<_>>()
            .join("&");
        format!("{base}{sep}{qs}")
    }

    fn is_json(body: &str) -> bool {
        matches!(body.trim_start().chars().next(), Some('{') | Some('['))
    }

    fn esc_shell(s: &str) -> String {
        s.replace('\'', "'\\''")
    }

    fn esc_dq(s: &str) -> String {
        s.replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
            .replace('\r', "\\r")
    }

    fn esc_rust(s: &str) -> String {
        s.replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
            .replace('\r', "\\r")
    }

    fn to_curl(req: &HttpRequest) -> String {
        let mut parts = vec![format!(
            "curl -X {} \"{}\"",
            req.method,
            Self::esc_dq(&Self::full_url(req))
        )];
        for (k, v) in &req.headers {
            parts.push(format!("-H \"{}: {}\"", Self::esc_dq(k), Self::esc_dq(v)));
        }
        match &req.auth {
            AuthPayload::None => {}
            AuthPayload::Basic { username, password } => {
                parts.push(format!(
                    "-u \"{}:{}\"",
                    Self::esc_dq(username),
                    Self::esc_dq(password)
                ));
            }
            AuthPayload::Bearer { token } => {
                parts.push(format!(
                    "-H \"Authorization: Bearer {}\"",
                    Self::esc_dq(token)
                ));
            }
        }
        if !req.body.is_empty() {
            parts.push(format!("--data-raw '{}'", Self::esc_shell(&req.body)));
        }
        parts.join(" \\\n  ")
    }

    fn fetch_headers(req: &HttpRequest) -> Vec<String> {
        let mut lines = Vec::new();
        for (k, v) in &req.headers {
            lines.push(format!(
                "    \"{}\": \"{}\",",
                Self::esc_dq(k),
                Self::esc_dq(v)
            ));
        }
        match &req.auth {
            AuthPayload::None => {}
            AuthPayload::Basic { username, password } => {
                lines.push(format!(
                    "    \"Authorization\": \"Basic \" + btoa(\"{}:{}\"),",
                    Self::esc_dq(username),
                    Self::esc_dq(password)
                ));
            }
            AuthPayload::Bearer { token } => {
                lines.push(format!(
                    "    \"Authorization\": \"Bearer {}\",",
                    Self::esc_dq(token)
                ));
            }
        }
        lines
    }

    fn to_fetch(req: &HttpRequest) -> String {
        let mut out = format!(
            "fetch(\"{}\", {{\n  method: \"{}\",",
            Self::esc_dq(&Self::full_url(req)),
            req.method
        );
        let headers = Self::fetch_headers(req);
        if !headers.is_empty() {
            out.push_str("\n  headers: {\n");
            out.push_str(&headers.join("\n"));
            out.push_str("\n  },");
        }
        if !req.body.is_empty() {
            out.push_str(&format!("\n  body: \"{}\",", Self::esc_dq(&req.body)));
        }
        out.push_str("\n});");
        out
    }

    fn to_axios(req: &HttpRequest) -> String {
        let mut out = format!(
            "axios({{\n  method: \"{}\",\n  url: \"{}\"",
            req.method.to_lowercase(),
            Self::esc_dq(&Self::full_url(req))
        );
        let headers = Self::fetch_headers(req);
        if !headers.is_empty() {
            out.push_str(",\n  headers: {\n");
            out.push_str(&headers.join("\n"));
            out.push_str("\n  }");
        }
        if let AuthPayload::Basic { username, password } = &req.auth {
            out.push_str(&format!(
                ",\n  auth: {{\n    username: \"{}\",\n    password: \"{}\",\n  }}",
                Self::esc_dq(username),
                Self::esc_dq(password)
            ));
        }
        if !req.body.is_empty() {
            if Self::is_json(&req.body) {
                out.push_str(&format!(",\n  data: {}", req.body.trim()));
            } else {
                out.push_str(&format!(",\n  data: \"{}\"", Self::esc_dq(&req.body)));
            }
        }
        out.push_str("\n});");
        out
    }

    fn to_python(req: &HttpRequest) -> String {
        let mut out = String::from("import requests\n\n");
        out.push_str(&format!(
            "url = \"{}\"\n",
            Self::esc_dq(&Self::full_url(req))
        ));
        if req.headers.is_empty() && !matches!(req.auth, AuthPayload::Bearer { .. }) {
            out.push_str("headers = {}\n");
        } else {
            out.push_str("headers = {\n");
            for (k, v) in &req.headers {
                out.push_str(&format!(
                    "    \"{}\": \"{}\",\n",
                    Self::esc_dq(k),
                    Self::esc_dq(v)
                ));
            }
            if let AuthPayload::Bearer { token } = &req.auth {
                out.push_str(&format!(
                    "    \"Authorization\": \"Bearer {}\",\n",
                    Self::esc_dq(token)
                ));
            }
            out.push_str("}\n");
        }
        let mut call = format!(
            "response = requests.request(\"{}\", url, headers=headers",
            req.method
        );
        if !req.body.is_empty() {
            if Self::is_json(&req.body) {
                out.push_str(&format!("payload = {}\n", req.body.trim()));
                call.push_str(", json=payload");
            } else {
                out.push_str(&format!("data = \"{}\"\n", Self::esc_dq(&req.body)));
                call.push_str(", data=data");
            }
        }
        if let AuthPayload::Basic { username, password } = &req.auth {
            call.push_str(&format!(
                ", auth=(\"{}\", \"{}\")",
                Self::esc_dq(username),
                Self::esc_dq(password)
            ));
        }
        call.push_str(")");
        out.push_str(&call);
        out.push_str("\nprint(response.status_code)\nprint(response.text)\n");
        out
    }

    fn to_rust(req: &HttpRequest) -> String {
        let url = Self::esc_rust(&Self::full_url(req));
        let start = match req.method.to_uppercase().as_str() {
            "GET" => format!("client.get(\"{url}\")"),
            "POST" => format!("client.post(\"{url}\")"),
            "PUT" => format!("client.put(\"{url}\")"),
            "PATCH" => format!("client.patch(\"{url}\")"),
            "DELETE" => format!("client.delete(\"{url}\")"),
            "HEAD" => format!("client.head(\"{url}\")"),
            _ => format!(
                "client.request(reqwest::Method::from_bytes(b\"{}\").unwrap(), \"{url}\")",
                req.method.to_uppercase()
            ),
        };
        let mut out = String::from("let client = reqwest::blocking::Client::new();\n");
        out.push_str("let res = ");
        out.push_str(&start);
        out.push('\n');
        for (k, v) in &req.headers {
            out.push_str(&format!(
                "    .header(\"{}\", \"{}\")\n",
                Self::esc_rust(k),
                Self::esc_rust(v)
            ));
        }
        match &req.auth {
            AuthPayload::None => {}
            AuthPayload::Basic { username, password } => {
                out.push_str(&format!(
                    "    .basic_auth(\"{}\", Some(\"{}\"))\n",
                    Self::esc_rust(username),
                    Self::esc_rust(password)
                ));
            }
            AuthPayload::Bearer { token } => {
                out.push_str(&format!(
                    "    .bearer_auth(\"{}\")\n",
                    Self::esc_rust(token)
                ));
            }
        }
        if !req.body.is_empty() {
            out.push_str(&format!("    .body(\"{}\")\n", Self::esc_rust(&req.body)));
        }
        out.push_str("    .send()?;\nprintln!(\"{}\", res.text()?);\n");
        out
    }
}
