pub struct CurlRequest;

impl CurlRequest {
    pub fn is_curl_command(command: &str) -> bool {
        command.trim_start().starts_with("curl ")
    }
    pub fn parse(command: &str) -> Result<serde_json::Value, anyhow::Error> {
        let args = Self::tokenize(command)?;
        if args.first().map(|s| s.as_str()) != Some("curl") {
            anyhow::bail!("not a curl command");
        }
        let mut url = String::new();
        let mut method: Option<String> = None;
        let mut data: Vec<String> = vec![];
        let mut headers: Vec<(String, String)> = vec![];
        let mut query: Vec<(String, String)> = vec![];
        let mut user: Option<(String, String)> = None;
        let mut bearer: Option<String> = None;
        let (mut is_get, mut json_ct) = (false, false);
        let mut i = 1;
        while i < args.len() {
            match args[i].as_str() {
                "-X" | "--request" => {
                    method = Some(Self::next(&args, &mut i)?);
                }
                "--url" => {
                    url = Self::next(&args, &mut i)?;
                }
                "-H" | "--header" => {
                    let h = Self::next(&args, &mut i)?;
                    if let Some((k, v)) = h.split_once(':') {
                        let (k, v) = (k.trim().to_string(), v.trim().to_string());
                        if k.eq_ignore_ascii_case("authorization") && v.starts_with("Bearer ") {
                            bearer = Some(v[7..].to_string());
                        } else {
                            headers.push((k, v));
                        }
                    }
                }
                "-d" | "--data" | "--data-raw" | "--data-binary" | "--data-ascii" => {
                    data.push(Self::next(&args, &mut i)?);
                }
                "--json" => {
                    json_ct = true;
                    data.push(Self::next(&args, &mut i)?);
                }
                "--data-urlencode" => {
                    let d = Self::next(&args, &mut i)?;
                    let d = d.strip_prefix('=').unwrap_or(&d).to_string();
                    if is_get {
                        query.push(Self::split_kv(&d));
                    } else {
                        data.push(d);
                    }
                }
                "-u" | "--user" => {
                    let u = Self::next(&args, &mut i)?;
                    let (a, b) = u.split_once(':').unwrap_or((&u, ""));
                    user = Some((a.to_string(), b.to_string()));
                }
                "--oauth2-bearer" => {
                    bearer = Some(Self::next(&args, &mut i)?);
                }
                "-b" | "--cookie" => Self::push_cookie(&mut headers, Self::next(&args, &mut i)?),
                "-G" | "--get" => {
                    is_get = true;
                }
                s if s.starts_with('-') => {
                    // skip flag value if it takes one
                    if Self::takes_value(s) && !args.get(i + 1).is_some_and(|n| n.starts_with('-')) {
                        i += 1;
                    }
                }
                s if url.is_empty() => {
                    url = s.to_string();
                }
                _ => {}
            }
            i += 1;
        }
        if url.is_empty() {
            anyhow::bail!("curl: no URL found");
        }
        // split ?k=v off URL -> query rows
        if let Some((base, qs)) = url.clone().split_once('?') {
            url = base.to_string();
            for p in qs.split('&') {
                query.push(Self::split_kv(p));
            }
        }
        let body = data.join("&");
        let m = method.unwrap_or(if body.is_empty() { "GET" } else { "POST" }.into());
        if json_ct {
            Self::upsert_ct(&mut headers, "application/json");
        }
        let (auth_type, username, password, token) = match (user, bearer) {
            (Some((u, p)), _) => ("Basic", u, p, String::new()),
            (_, Some(t)) => ("Bearer", String::new(), String::new(), t),
            _ => ("None", String::new(), String::new(), String::new()),
        };
        let kv = |v: Vec<(String, String)>| {
            v.into_iter()
                .map(|(k, val)| serde_json::json!({"key": k, "value": val, "active": true}))
                .collect::<Vec<_>>()
        };
        Ok(serde_json::json!({
            "url": url, "method": m.to_uppercase(),
            "params": kv(query), "headers": kv(headers),
            "auth": {"auth_type": auth_type, "username": username, "password": password, "token": token},
            "body": {"body_type": if body.trim_start().starts_with('{') {"JSON"} else {"Text"}, "body": body}
        }))
    }

    fn tokenize(s: &str) -> anyhow::Result<Vec<String>> {
        let flat = s.replace("\\\r\n", " ").replace("\\\n", " ");
        let (mut out, mut cur, mut q) = (vec![], String::new(), None::<char>);
        let mut chs = flat.chars().peekable();
        while let Some(c) = chs.next() {
            match (q, c) {
                (None, '\'') | (None, '"') => q = Some(c),
                (Some(x), y) if x == y => q = None,
                (None, c) if c.is_whitespace() => {
                    if !cur.is_empty() {
                        out.push(std::mem::take(&mut cur));
                    }
                }
                (None, '\\') => {
                    if let Some(n) = chs.next() {
                        cur.push(n);
                    }
                }
                (Some(_), '\\') => {
                    if let Some(n) = chs.next() {
                        cur.push(match n {
                            'n' => '\n',
                            't' => '\t',
                            o => o,
                        });
                    }
                }
                (_, c) => cur.push(c),
            }
        }
        if !cur.is_empty() {
            out.push(cur);
        }
        Ok(out)
    }
    fn next(a: &[String], i: &mut usize) -> anyhow::Result<String> {
        *i += 1;
        a.get(*i)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("missing value"))
    }
    fn split_kv(s: &str) -> (String, String) {
        s.split_once('=')
            .map(|(k, v)| (k.into(), v.into()))
            .unwrap_or((s.into(), String::new()))
    }
    fn takes_value(f: &str) -> bool {
        matches!(
            f,
            "--connect-timeout"
                | "--max-time"
                | "--retry"
                | "-m"
                | "--proxy"
                | "-x"
                | "-U"
                | "--cert"
                | "-E"
        )
    }
    fn push_cookie(h: &mut Vec<(String, String)>, c: String) {
        match h.iter_mut().find(|(k, _)| k.eq_ignore_ascii_case("cookie")) {
            Some((_, v)) => {
                *v += &format!("; {c}");
            }
            None => h.push(("Cookie".into(), c)),
        }
    }
    fn upsert_ct(h: &mut Vec<(String, String)>, v: &str) {
        if !h
            .iter()
            .any(|(k, _)| k.eq_ignore_ascii_case("content-type"))
        {
            h.push(("Content-Type".into(), v.into()));
        }
    }
}
