//! The CDN hosts streams are served from, and which of them can be reached.

use super::models::{StreamBundle, StreamEntry};
use super::session::RezkaSession;
use reqwest::header::{CONTENT_TYPE, RANGE, REFERER};
use std::collections::{HashMap, HashSet};
use std::sync::{LazyLock, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

/// Hosts that serve the same files under the same paths. The provider names
/// one of them for every stream and offers no alternative, and when that one
/// stops answering the files are still there on the others.
const HOSTS: [&str; 11] = [
    "prx-cogent.ukrtelcdn.net",
    "prx2-cogent.ukrtelcdn.net",
    "prx3-cogent.ukrtelcdn.net",
    "prx4-cogent.ukrtelcdn.net",
    "prx5-cogent.ukrtelcdn.net",
    "prx6-cogent.ukrtelcdn.net",
    "prx-ams.ukrtelcdn.net",
    "stream.voidboost.cc",
    "stream.voidboost.top",
    "stream.voidboost.link",
    "stream.voidboost.club",
];

/// How long a host gets to start answering before it counts as down.
const TIMEOUT: Duration = Duration::from_secs(4);

/// How long a check of every host is trusted before the next one is due.
const CHECK_INTERVAL: Duration = Duration::from_secs(10 * 60);

/// What is known about the hosts, shared by every stream fetched.
#[derive(Default)]
struct Health {
    /// Hosts that did not answer when last asked. They are still tried, but
    /// playback no longer waits for them to time out.
    down: HashSet<String>,
    /// How long each host took to answer the last check of every host.
    latency: HashMap<String, Duration>,
    /// When the last check of every host finished.
    checked: Option<Instant>,
    checking: bool,
}

static HEALTH: LazyLock<Mutex<Health>> = LazyLock::new(Mutex::default);

fn lock(health: &Mutex<Health>) -> MutexGuard<'_, Health> {
    health.lock().unwrap_or_else(PoisonError::into_inner)
}

fn client(user_agent: &str) -> Option<reqwest::Client> {
    reqwest::Client::builder()
        .user_agent(user_agent)
        .connect_timeout(TIMEOUT)
        .timeout(TIMEOUT)
        .build()
        .map_err(|error| log::warn!("Could not build the CDN client: {}", error))
        .ok()
}

fn url_origin(url: &str) -> String {
    url::Url::parse(url)
        .map(|url| url.origin().ascii_serialization())
        .unwrap_or_default()
}

/// Follows `urls` with the same files on every other known host, so a player
/// that cannot reach the provider's choice has somewhere to go next.
pub(super) fn with_mirrors(urls: Vec<String>) -> Vec<String> {
    let mut all = urls.clone();
    for url in &urls {
        let Ok(parsed) = url::Url::parse(url) else {
            continue;
        };
        let Some(own_host) = parsed.host_str().filter(|host| HOSTS.contains(host)) else {
            continue;
        };
        for host in HOSTS.into_iter().filter(|host| *host != own_host) {
            let mut mirror = parsed.clone();
            if mirror.set_host(Some(host)).is_err() {
                continue;
            }
            let mirror = String::from(mirror);
            if !all.contains(&mirror) {
                all.push(mirror);
            }
        }
    }
    all
}

/// Starts checking every host in the background, unless a check is running
/// or the last one is recent enough. Does nothing outside a Tokio runtime.
pub(super) fn check_if_due(user_agent: &str) {
    let Ok(runtime) = tokio::runtime::Handle::try_current() else {
        return;
    };
    if !claim_check(&mut lock(&HEALTH), Instant::now()) {
        return;
    }
    let Some(client) = client(user_agent) else {
        lock(&HEALTH).checking = false;
        return;
    };
    let origins = HOSTS.map(|host| format!("https://{host}"));
    runtime.spawn(async move { check(&client, &origins, &HEALTH).await });
}

/// Marks a check as running if one is due, and says whether it was.
fn claim_check(health: &mut Health, now: Instant) -> bool {
    let recent = health
        .checked
        .is_some_and(|checked| now.duration_since(checked) < CHECK_INTERVAL);
    if health.checking || recent {
        return false;
    }
    health.checking = true;
    true
}

/// Asks every host for its front page. No stream is at hand to ask for, so
/// this only shows which hosts can be reached and how fast; whether one
/// serves a given file is still settled when it is played.
async fn check(client: &reqwest::Client, origins: &[String], health: &Mutex<Health>) {
    let mut probes = tokio::task::JoinSet::new();
    for origin in origins {
        let request = client.get(origin);
        let origin = origin.clone();
        probes.spawn(async move {
            let started = Instant::now();
            let answered = request.send().await.is_ok();
            (origin, answered.then(|| started.elapsed()))
        });
    }
    let results = probes.join_all().await;

    let mut health = lock(health);
    for (origin, latency) in results {
        match latency {
            Some(latency) => {
                health.down.remove(&origin);
                health.latency.insert(origin, latency);
            }
            None => {
                health.latency.remove(&origin);
                health.down.insert(origin);
            }
        }
    }
    log::debug!("CDN hosts down: {:?}", health.down);
    health.checked = Some(Instant::now());
    health.checking = false;
}

/// Puts a host that serves the stream at the front of every stream's URLs,
/// the others after it from the fastest, and the ones known to be down last.
///
/// Players other than the Android one only ever see the first URL, so one on
/// a host that is down would fail however many alternatives follow it; the
/// Android one walks the rest in order, waiting out each host that is down.
pub(super) async fn prefer_reachable(session: &RezkaSession, bundle: &mut StreamBundle) {
    check_if_due(session.user_agent());
    let Some(client) = client(session.user_agent()) else {
        return;
    };
    promote_reachable(&client, session.referer(), &HEALTH, &mut bundle.streams).await;
}

async fn promote_reachable(
    client: &reqwest::Client,
    referer: &str,
    health: &Mutex<Health>,
    streams: &mut [StreamEntry],
) {
    // Every quality lives on the same hosts, so one of them speaks for all;
    // premium ones may refuse an account without a subscription.
    let Some(entry) = streams
        .iter()
        .filter(|entry| entry.urls.len() > 1)
        .min_by_key(|entry| entry.is_premium)
    else {
        return;
    };
    // One request per host is enough to tell whether it serves the stream.
    let mut origins = HashSet::new();
    let candidates: Vec<String> = entry
        .urls
        .iter()
        .filter(|url| origins.insert(url_origin(url)))
        .cloned()
        .collect();
    if candidates.len() < 2 {
        return;
    }
    let Some(index) = reachable_candidate(client, referer, health, &candidates).await else {
        log::warn!("No stream host answered; keeping the provider's order");
        return;
    };
    let origin = url_origin(&candidates[index]);
    if index > 0 {
        log::info!(
            "Stream host {} did not answer, playing from {}",
            url_origin(&candidates[0]),
            origin
        );
    }
    let (down, latency) = {
        let health = lock(health);
        (health.down.clone(), health.latency.clone())
    };
    for entry in streams.iter_mut() {
        // A stable sort, so the provider's order holds among equals.
        entry.urls.sort_by_key(|url| {
            let url_origin = url_origin(url);
            let group = if url_origin == origin {
                0
            } else if down.contains(&url_origin) {
                2
            } else {
                1
            };
            (
                group,
                latency.get(&url_origin).copied().unwrap_or(Duration::MAX),
            )
        });
    }
}

/// Picks the candidate playback should start from: the first one whenever
/// it answers, since that is the provider's choice, and otherwise whichever
/// of the rest answered first. A first candidate on a host known to be down
/// only wins by answering before all the others.
async fn reachable_candidate(
    client: &reqwest::Client,
    referer: &str,
    health: &Mutex<Health>,
    candidates: &[String],
) -> Option<usize> {
    let first_origin = url_origin(&candidates[0]);
    let mut first_failed = lock(health).down.contains(&first_origin);

    let mut probes = tokio::task::JoinSet::new();
    for (index, url) in candidates.iter().enumerate() {
        let request = client
            .get(url)
            .header(REFERER, referer)
            .header(RANGE, "bytes=0-0");
        probes.spawn(async move {
            // A host blocked on the way answers with a web page, not the file.
            let answered = request.send().await.is_ok_and(|response| {
                let is_page = response
                    .headers()
                    .get(CONTENT_TYPE)
                    .and_then(|value| value.to_str().ok())
                    .is_some_and(|value| value.starts_with("text/html"));
                response.status().is_success() && !is_page
            });
            (index, answered)
        });
    }

    let mut first_answer = None;
    while let Some(result) = probes.join_next().await {
        let Ok((index, answered)) = result else {
            continue;
        };
        if index == 0 {
            let mut health = lock(health);
            if answered {
                health.down.remove(&first_origin);
                return Some(0);
            }
            health.down.insert(first_origin.clone());
            first_failed = true;
        } else if answered && first_answer.is_none() {
            first_answer = Some(index);
        }
        if first_failed && first_answer.is_some() {
            break;
        }
    }
    first_answer
}

#[cfg(test)]
mod tests {
    use super::super::models::StreamEntry;
    use super::super::test_support::{read_request, respond};
    use super::{check, claim_check, promote_reachable, with_mirrors};
    use super::{Health, CHECK_INTERVAL, HOSTS};
    use std::collections::{HashMap, HashSet};
    use std::net::TcpListener;
    use std::sync::Mutex;
    use std::time::{Duration, Instant};

    #[test]
    fn offers_a_url_on_every_other_known_host() {
        let urls = with_mirrors(vec![
            "https://prx-ams.ukrtelcdn.net/s/a/720.mp4:hls:manifest.m3u8".to_string(),
            "https://prx-ams.ukrtelcdn.net/s/a/720.mp4".to_string(),
        ]);

        assert_eq!(
            &urls[..2],
            [
                "https://prx-ams.ukrtelcdn.net/s/a/720.mp4:hls:manifest.m3u8",
                "https://prx-ams.ukrtelcdn.net/s/a/720.mp4",
            ]
        );
        assert_eq!(urls.len(), 2 * HOSTS.len());
        assert!(
            urls.contains(&"https://stream.voidboost.cc/s/a/720.mp4:hls:manifest.m3u8".to_string())
        );
        assert!(urls.contains(&"https://prx-cogent.ukrtelcdn.net/s/a/720.mp4".to_string()));
    }

    #[test]
    fn leaves_urls_on_other_hosts_alone() {
        let urls = with_mirrors(vec!["https://cdn.example.com/720.mp4".to_string()]);
        assert_eq!(urls, ["https://cdn.example.com/720.mp4"]);
    }

    #[test]
    fn checks_again_only_once_the_last_check_is_old() {
        let now = Instant::now();
        let mut health = Health::default();
        assert!(claim_check(&mut health, now));
        assert!(!claim_check(&mut health, now), "a check is running");

        health.checking = false;
        health.checked = Some(now);
        assert!(!claim_check(&mut health, now + CHECK_INTERVAL / 2));
        assert!(claim_check(&mut health, now + CHECK_INTERVAL));
    }

    /// A host that answers every request with a success.
    fn live_host() -> String {
        serving("video/mp4", Duration::ZERO)
    }

    /// A host that answers every request with a success after `delay`.
    fn slow_host(delay: Duration) -> String {
        serving("video/mp4", delay)
    }

    fn serving(content_type: &'static str, delay: Duration) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        std::thread::spawn(move || {
            for mut stream in listener.incoming().flatten() {
                read_request(&mut stream);
                std::thread::sleep(delay);
                respond(&mut stream, content_type, "x");
            }
        });
        origin
    }

    /// A host nothing is listening on.
    fn dead_host() -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        format!("http://{}", listener.local_addr().unwrap())
    }

    fn entry(quality: &str, hosts: &[&str]) -> StreamEntry {
        StreamEntry {
            quality: quality.to_string(),
            is_premium: false,
            urls: hosts
                .iter()
                .map(|host| format!("{host}/{quality}.mp4"))
                .collect(),
        }
    }

    fn down(hosts: &[&str]) -> Mutex<Health> {
        Mutex::new(Health {
            down: hosts.iter().map(|host| host.to_string()).collect(),
            ..Health::default()
        })
    }

    async fn promote(health: &Mutex<Health>, streams: &mut [StreamEntry]) {
        promote_reachable(
            &reqwest::Client::new(),
            "https://hdrzk.org",
            health,
            streams,
        )
        .await;
    }

    #[tokio::test]
    async fn check_marks_hosts_down_and_times_the_rest() {
        let (dead, live) = (dead_host(), live_host());
        let health = down(&[&live]);
        {
            let mut health = health.lock().unwrap();
            health.checking = true;
        }

        check(
            &reqwest::Client::new(),
            &[dead.clone(), live.clone()],
            &health,
        )
        .await;

        let health = health.lock().unwrap();
        assert_eq!(health.down, HashSet::from([dead]));
        assert!(health.latency.contains_key(&live));
        assert!(health.checked.is_some());
        assert!(!health.checking);
    }

    #[tokio::test]
    async fn plays_from_a_host_that_answers_when_the_first_does_not() {
        let (dead, live) = (dead_host(), live_host());
        let health = Mutex::default();
        let mut streams = vec![
            entry("1080p", &[&dead, &live]),
            entry("720p", &[&dead, &live]),
        ];

        promote(&health, &mut streams).await;

        for stream in &streams {
            assert_eq!(stream.urls[0], format!("{live}/{}.mp4", stream.quality));
            assert_eq!(stream.urls[1], format!("{dead}/{}.mp4", stream.quality));
        }
        assert!(health.lock().unwrap().down.contains(&dead));
    }

    #[tokio::test]
    async fn keeps_the_providers_host_while_it_answers() {
        let (first, second) = (live_host(), live_host());
        let health = Mutex::default();
        let mut streams = vec![entry("720p", &[&first, &second])];

        promote(&health, &mut streams).await;

        assert_eq!(streams[0].urls[0], format!("{first}/720p.mp4"));
    }

    #[tokio::test]
    async fn keeps_the_order_when_no_host_answers() {
        let (first, second) = (dead_host(), dead_host());
        let health = Mutex::default();
        let mut streams = vec![entry("720p", &[&first, &second])];

        promote(&health, &mut streams).await;

        assert_eq!(streams[0].urls[0], format!("{first}/720p.mp4"));
    }

    #[tokio::test]
    async fn falls_back_to_faster_hosts_first_and_to_hosts_that_are_down_last() {
        let (dead, live) = (dead_host(), live_host());
        let slower = slow_host(Duration::from_secs(1));
        let faster = slow_host(Duration::from_secs(1));
        let health = Mutex::new(Health {
            latency: HashMap::from([
                (slower.clone(), Duration::from_millis(50)),
                (faster.clone(), Duration::from_millis(10)),
            ]),
            ..Health::default()
        });
        let mut streams = vec![entry("720p", &[&dead, &slower, &faster, &live])];

        promote(&health, &mut streams).await;

        assert_eq!(
            streams[0].urls,
            [
                format!("{live}/720p.mp4"),
                format!("{faster}/720p.mp4"),
                format!("{slower}/720p.mp4"),
                format!("{dead}/720p.mp4"),
            ]
        );
    }

    #[tokio::test]
    async fn takes_a_host_answering_with_a_web_page_for_down() {
        let (page, live) = (
            serving("text/html; charset=utf-8", Duration::ZERO),
            live_host(),
        );
        let health = Mutex::default();
        let mut streams = vec![entry("720p", &[&page, &live])];

        promote(&health, &mut streams).await;

        assert_eq!(streams[0].urls[0], format!("{live}/720p.mp4"));
    }

    #[tokio::test]
    async fn stops_waiting_for_a_host_that_did_not_answer_before() {
        let (slow, live) = (slow_host(Duration::from_secs(1)), live_host());
        let health = down(&[&slow]);
        let mut streams = vec![entry("720p", &[&slow, &live])];

        promote(&health, &mut streams).await;

        assert_eq!(streams[0].urls[0], format!("{live}/720p.mp4"));
    }

    #[tokio::test]
    async fn trusts_a_host_again_once_it_answers() {
        let (first, second) = (live_host(), dead_host());
        let health = down(&[&first]);
        let mut streams = vec![entry("720p", &[&first, &second])];

        promote(&health, &mut streams).await;

        assert_eq!(streams[0].urls[0], format!("{first}/720p.mp4"));
        assert!(health.lock().unwrap().down.is_empty());
    }
}
