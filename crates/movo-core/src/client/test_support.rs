//! HTTP test fixtures shared by this module's test suites.
#![cfg(test)]

use super::models::UserProfile;
use super::session::RezkaSession;
use super::{AccountState, RezkaClient};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::{Arc, RwLock};

/// Reads one request off `stream` and returns it as text.
pub(crate) fn read_request(stream: &mut TcpStream) -> String {
    let mut request = Vec::new();
    let mut buffer = [0; 4096];
    loop {
        let read = stream.read(&mut buffer).unwrap();
        if read == 0 {
            break;
        }
        request.extend_from_slice(&buffer[..read]);
        let Some(headers_end) = request.windows(4).position(|part| part == b"\r\n\r\n") else {
            continue;
        };
        let headers = String::from_utf8_lossy(&request[..headers_end]);
        let content_length = headers
            .lines()
            .find_map(|line| {
                line.split_once(':').and_then(|(name, value)| {
                    name.eq_ignore_ascii_case("content-length")
                        .then(|| value.trim().parse::<usize>().ok())
                        .flatten()
                })
            })
            .unwrap_or(0);
        if request.len() >= headers_end + 4 + content_length {
            break;
        }
    }
    String::from_utf8(request).unwrap()
}

/// Answers `stream` with a 200 OK carrying `body`, so the caller sees a
/// server that replied successfully.
pub(crate) fn respond(stream: &mut TcpStream, content_type: &str, body: &str) {
    write!(
        stream,
        "HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
    .unwrap();
}

/// A client already signed in as user "42", pointed at a local test server.
pub(crate) fn authenticated_test_client(base_url: &str) -> RezkaClient {
    let session = RezkaSession::new_for_test(base_url);
    session.authenticate_for_test("42");
    RezkaClient {
        session,
        hidden_countries: Arc::default(),
        account: Arc::new(RwLock::new(AccountState {
            user: Some(UserProfile {
                user_id: "42".to_string(),
                username: "Tester".to_string(),
                is_logged_in: true,
                is_vip: false,
                email: None,
                avatar_url: None,
                premium_days: None,
                is_session_persistent: false,
            }),
            generation: 0,
        })),
    }
}
