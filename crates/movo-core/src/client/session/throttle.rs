//! Keeps requests under the provider's request limit.
//!
//! The provider answers a burst of about five requests at once and then
//! about one and a half a second, and turns away whatever goes over with
//! nginx's own 404 page instead of a 429.

use std::sync::{Mutex, PoisonError};
use std::time::Duration;
use tokio::time::Instant;

/// Requests that may go out at once, one short of the provider's burst.
const BURST: f64 = 4.0;

/// How often another request may go out once the burst is spent, a little
/// slower than the provider's rate.
const INTERVAL: Duration = Duration::from_millis(700);

pub(super) struct Throttle {
    /// Spent on each request and earned back one per interval; `None` lets
    /// every request through at once.
    interval: Option<Duration>,
    state: Mutex<(f64, Instant)>,
}

impl Throttle {
    pub(super) fn provider() -> Self {
        Self::with_interval(Some(INTERVAL))
    }

    /// For sessions against a local test server, which has no limit.
    #[cfg(test)]
    pub(super) fn unlimited() -> Self {
        Self::with_interval(None)
    }

    fn with_interval(interval: Option<Duration>) -> Self {
        Self {
            interval,
            state: Mutex::new((BURST, Instant::now())),
        }
    }

    /// Waits until another request may go out, and counts it.
    pub(super) async fn acquire(&self) {
        let Some(interval) = self.interval else {
            return;
        };
        loop {
            let wait = {
                let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
                let (allowance, updated) = &mut *state;
                let now = Instant::now();
                let earned = now.duration_since(*updated).as_secs_f64() / interval.as_secs_f64();
                *allowance = (*allowance + earned).min(BURST);
                *updated = now;
                if *allowance >= 1.0 {
                    *allowance -= 1.0;
                    return;
                }
                interval.mul_f64(1.0 - *allowance)
            };
            tokio::time::sleep(wait).await;
        }
    }

    /// Spends whatever is left. The provider has just turned a request away,
    /// so its own count is full whatever this one says: someone else on the
    /// same address may be using it too.
    pub(super) fn exhaust(&self) {
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        *state = (0.0, Instant::now());
    }
}

/// Whether a reply is the provider's limit turning the request away. It
/// never reached the site, so sending it again cannot apply it twice.
pub(super) fn is_refusal(status: reqwest::StatusCode, body: &str) -> bool {
    status == reqwest::StatusCode::NOT_FOUND && body.contains("<center>nginx</center>")
}

#[cfg(test)]
mod tests {
    use super::{is_refusal, Throttle, BURST};
    use reqwest::StatusCode;
    use std::time::Duration;
    use tokio::time::Instant;

    const INTERVAL: Duration = Duration::from_millis(40);

    /// Room for the clock: the allowance keeps earning while the test runs.
    const SLACK: Duration = Duration::from_millis(5);

    #[tokio::test]
    async fn lets_a_burst_through_and_then_paces_the_rest() {
        let throttle = Throttle::with_interval(Some(INTERVAL));
        let started = Instant::now();

        for _ in 0..BURST as u32 {
            throttle.acquire().await;
        }
        assert!(started.elapsed() < INTERVAL, "{:?}", started.elapsed());

        throttle.acquire().await;
        throttle.acquire().await;
        assert!(
            started.elapsed() >= INTERVAL * 2 - SLACK,
            "{:?}",
            started.elapsed()
        );
    }

    #[tokio::test]
    async fn waits_a_full_interval_after_a_refusal() {
        let throttle = Throttle::with_interval(Some(INTERVAL));
        throttle.exhaust();
        let started = Instant::now();

        throttle.acquire().await;

        assert!(
            started.elapsed() >= INTERVAL - SLACK,
            "{:?}",
            started.elapsed()
        );
    }

    #[test]
    fn recognises_only_the_limits_own_page() {
        let page = "<html>\r\n<head><title>404 Not Found</title></head>\r\n<body>\r\n\
                    <center><h1>404 Not Found</h1></center>\r\n<hr><center>nginx</center>\r\n</body>\r\n</html>";
        assert!(is_refusal(StatusCode::NOT_FOUND, page));
        assert!(!is_refusal(
            StatusCode::NOT_FOUND,
            "<html>Страница не найдена</html>"
        ));
        assert!(!is_refusal(StatusCode::FORBIDDEN, page));
    }
}
