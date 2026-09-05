//! A stable code for each kind of failure the client reports.
//!
//! Errors travel through the client as messages written for a log, so a front
//! end that wants to say something in the user's language has nothing to key
//! on. This reads the message back into one of a few kinds every front end can
//! name in its own words, leaving the message itself for the log and for the
//! rare failure that has no kind.

/// The kind of a failure, as a front end would explain it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    /// The provider did not answer in time.
    Timeout,
    /// The request never reached the provider, or its reply never arrived.
    Network,
    /// The provider answered with a failure status or a challenge it would not clear.
    Provider,
    /// The stored session is no longer accepted.
    Session,
    /// The provider's reply could not be read.
    Malformed,
    /// Anything else; the message is all there is.
    Other,
}

impl ErrorKind {
    /// The code a front end matches on.
    pub fn code(self) -> &'static str {
        match self {
            Self::Timeout => "timeout",
            Self::Network => "network",
            Self::Provider => "provider",
            Self::Session => "session",
            Self::Malformed => "malformed",
            Self::Other => "other",
        }
    }
}

/// Reads the kind of failure out of one of the client's error messages.
pub fn classify(message: &str) -> ErrorKind {
    let lower = message.to_ascii_lowercase();
    let has = |needle: &str| lower.contains(needle);
    if has("timed out") || has("timeout") {
        ErrorKind::Timeout
    } else if has("request failed: http ") || has("anubis") {
        ErrorKind::Provider
    } else if has("request failed")
        || has("failed to send")
        || has("failed to read http response")
        || has("http response exceeded")
    {
        ErrorKind::Network
    } else if has("session") && (has("rejected") || has("expired") || has("no longer valid") || has("invalid"))
    {
        ErrorKind::Session
    } else if has("malformed") || has("failed to parse") || has("invalid history page") {
        ErrorKind::Malformed
    } else {
        ErrorKind::Other
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_the_transport_failures() {
        assert_eq!(classify("Timed out"), ErrorKind::Timeout);
        assert_eq!(
            classify("HTTP GET request failed: error sending request: operation timed out"),
            ErrorKind::Timeout
        );
        assert_eq!(
            classify("HTTP POST request failed: error sending request: dns error"),
            ErrorKind::Network
        );
        assert_eq!(
            classify("Failed to read HTTP response: connection reset"),
            ErrorKind::Network
        );
    }

    #[test]
    fn names_what_the_provider_said() {
        assert_eq!(
            classify("catalog request failed: HTTP 503; content-type text/html; HTML body (120 bytes)"),
            ErrorKind::Provider
        );
        assert_eq!(
            classify("Anubis clearance cookie not obtained after solving for https://x"),
            ErrorKind::Provider
        );
        assert_eq!(classify("Comments response was malformed"), ErrorKind::Malformed);
        assert_eq!(classify("Failed to parse stream JSON: eof"), ErrorKind::Malformed);
    }

    #[test]
    fn names_a_dead_session_and_leaves_the_rest() {
        assert_eq!(classify("Stored session has expired"), ErrorKind::Session);
        assert_eq!(classify("Authenticated session is no longer valid"), ErrorKind::Session);
        assert_eq!(classify("Authenticated session was rejected by the official provider"), ErrorKind::Session);
        assert_eq!(classify("Rating must be between 1 and 10"), ErrorKind::Other);
        assert_eq!(ErrorKind::Other.code(), "other");
    }
}
