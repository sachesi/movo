//! The little HTTP the client speaks: one request per connection, form bodies
//! either multipart or url-encoded.

use std::collections::HashMap;
use std::io::{self, Read, Write};
use std::net::TcpStream;

const MAX_REQUEST_BYTES: usize = 1 << 20;

#[derive(Debug, Clone)]
pub struct Request {
    pub method: String,
    pub path: String,
    pub query: HashMap<String, String>,
    pub cookies: HashMap<String, String>,
    pub form: HashMap<String, String>,
    pub referer: Option<String>,
    pub user_agent: Option<String>,
}

pub fn read_request(stream: &mut TcpStream) -> io::Result<Option<Request>> {
    let mut raw = Vec::new();
    let mut buffer = [0; 8192];
    let (headers_end, content_length) = loop {
        let read = stream.read(&mut buffer)?;
        if read == 0 {
            return Ok(None);
        }
        raw.extend_from_slice(&buffer[..read]);
        if raw.len() > MAX_REQUEST_BYTES {
            return Ok(None);
        }
        if let Some(end) = raw.windows(4).position(|part| part == b"\r\n\r\n") {
            break (end, content_length(&raw[..end]));
        }
    };
    while raw.len() < headers_end + 4 + content_length {
        let read = stream.read(&mut buffer)?;
        if read == 0 {
            return Ok(None);
        }
        raw.extend_from_slice(&buffer[..read]);
    }

    let head = String::from_utf8_lossy(&raw[..headers_end]).into_owned();
    let body = &raw[headers_end + 4..];
    let mut lines = head.split("\r\n");
    let mut request_line = lines.next().unwrap_or_default().split_whitespace();
    let method = request_line.next().unwrap_or_default().to_string();
    let target = request_line.next().unwrap_or("/");
    let (path, query) = target.split_once('?').unwrap_or((target, ""));

    let mut request = Request {
        method,
        path: percent_decode(path),
        query: parse_urlencoded(query),
        cookies: HashMap::new(),
        form: HashMap::new(),
        referer: None,
        user_agent: None,
    };
    let mut content_type = String::new();
    for line in lines {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        let value = value.trim();
        match name.to_ascii_lowercase().as_str() {
            "cookie" => {
                for pair in value.split(';') {
                    if let Some((name, value)) = pair.trim().split_once('=') {
                        request.cookies.insert(name.to_string(), value.to_string());
                    }
                }
            }
            "content-type" => content_type = value.to_string(),
            "referer" => request.referer = Some(value.to_string()),
            "user-agent" => request.user_agent = Some(value.to_string()),
            _ => {}
        }
    }
    request.form = parse_form(&content_type, body);
    Ok(Some(request))
}

fn content_length(head: &[u8]) -> usize {
    String::from_utf8_lossy(head)
        .split("\r\n")
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse().ok())
                .flatten()
        })
        .unwrap_or(0)
}

fn parse_form(content_type: &str, body: &[u8]) -> HashMap<String, String> {
    if let Some(boundary) = content_type.split("boundary=").nth(1) {
        return parse_multipart(&String::from_utf8_lossy(body), boundary.trim_matches('"'));
    }
    parse_urlencoded(&String::from_utf8_lossy(body))
}

fn parse_multipart(body: &str, boundary: &str) -> HashMap<String, String> {
    let delimiter = format!("--{boundary}");
    body.split(&delimiter)
        .filter_map(|part| {
            let (head, value) = part.split_once("\r\n\r\n")?;
            let name = head.split("name=\"").nth(1)?.split('"').next()?;
            Some((name.to_string(), value.trim_end_matches("\r\n").to_string()))
        })
        .collect()
}

pub fn parse_urlencoded(text: &str) -> HashMap<String, String> {
    text.split('&')
        .filter(|pair| !pair.is_empty())
        .map(|pair| {
            let (name, value) = pair.split_once('=').unwrap_or((pair, ""));
            (percent_decode(name), percent_decode(value))
        })
        .collect()
}

fn percent_decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut at = 0;
    while at < bytes.len() {
        match bytes[at] {
            b'+' => decoded.push(b' '),
            b'%' => {
                let value = text
                    .get(at + 1..at + 3)
                    .and_then(|hex| u8::from_str_radix(hex, 16).ok());
                if let Some(value) = value {
                    decoded.push(value);
                    at += 2;
                } else {
                    decoded.push(b'%');
                }
            }
            byte => decoded.push(byte),
        }
        at += 1;
    }
    String::from_utf8_lossy(&decoded).into_owned()
}

#[derive(Debug, Clone)]
pub struct Response {
    pub status: u16,
    pub content_type: &'static str,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl Response {
    pub fn new(status: u16, content_type: &'static str, body: impl Into<Vec<u8>>) -> Self {
        Self {
            status,
            content_type,
            headers: Vec::new(),
            body: body.into(),
        }
    }

    pub fn html(body: impl Into<Vec<u8>>) -> Self {
        Self::new(200, "text/html; charset=utf-8", body)
    }

    pub fn json(value: &serde_json::Value) -> Self {
        Self::new(200, "application/json", value.to_string())
    }

    pub fn not_found() -> Self {
        Self::new(
            404,
            "text/html; charset=utf-8",
            "<html>Страница не найдена</html>",
        )
    }

    pub fn with_header(mut self, name: &str, value: impl Into<String>) -> Self {
        self.headers.push((name.to_string(), value.into()));
        self
    }

    pub fn write_to(&self, stream: &mut TcpStream) -> io::Result<()> {
        let mut head = format!(
            "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n",
            self.status,
            reason(self.status),
            self.content_type,
            self.body.len()
        );
        for (name, value) in &self.headers {
            head.push_str(&format!("{name}: {value}\r\n"));
        }
        head.push_str("\r\n");
        stream.write_all(head.as_bytes())?;
        stream.write_all(&self.body)
    }
}

fn reason(status: u16) -> &'static str {
    match status {
        200 => "OK",
        206 => "Partial Content",
        302 => "Found",
        403 => "Forbidden",
        404 => "Not Found",
        429 => "Too Many Requests",
        500 => "Internal Server Error",
        502 => "Bad Gateway",
        503 => "Service Unavailable",
        _ => "Status",
    }
}
