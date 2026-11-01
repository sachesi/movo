//! A local stand-in for the streaming provider. It serves an invented library
//! in the site's markup, keeps an account's writes, and misbehaves on request
//! so tests and headless runs can reach the failure paths the real site only
//! shows on a bad day.

pub mod catalog;
mod http;
mod pages;
mod routes;
pub mod state;

pub use http::{Request, Response};

use http::read_request;
use state::State;
use std::io;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread;
use std::time::Duration;

/// How a matching request goes wrong.
#[derive(Debug, Clone)]
pub enum FaultKind {
    /// Replies with this status and an empty body.
    Status(u16),
    /// The answer takes this long to start.
    Delay(Duration),
    /// A 200 whose body is not what the endpoint promises.
    Garbage,
    /// A 200 with nothing in it.
    Empty,
    /// The connection closes without a reply.
    Drop,
    /// The request takes effect, then the connection closes without a reply.
    LoseReply,
    /// The request limit's nginx page, which the client resends past.
    LimitRefusal,
    /// Every signed-in page answers as if the session had expired.
    SessionExpired,
}

#[derive(Debug, Clone)]
pub struct Fault {
    /// Applies to requests whose path contains this.
    pub path_contains: String,
    pub kind: FaultKind,
    /// Applies this many times, or forever when `None`.
    pub remaining: Option<u32>,
}

impl Fault {
    pub fn always(path_contains: impl Into<String>, kind: FaultKind) -> Self {
        Self {
            path_contains: path_contains.into(),
            kind,
            remaining: None,
        }
    }

    pub fn times(path_contains: impl Into<String>, kind: FaultKind, times: u32) -> Self {
        Self {
            remaining: Some(times),
            ..Self::always(path_contains, kind)
        }
    }
}

pub(crate) struct Shared {
    pub(crate) state: Mutex<State>,
    pub(crate) faults: Mutex<Vec<Fault>>,
    pub(crate) log: Mutex<Vec<Request>>,
    pub(crate) anubis_difficulty: Mutex<Option<usize>>,
    pub(crate) stopped: AtomicBool,
}

pub(crate) fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

pub struct FakeProvider {
    address: SocketAddr,
    shared: Arc<Shared>,
}

impl FakeProvider {
    /// Starts on a free loopback port.
    pub fn start() -> io::Result<Self> {
        Self::bind("127.0.0.1:0")
    }

    pub fn bind(address: &str) -> io::Result<Self> {
        let listener = TcpListener::bind(address)?;
        let address = listener.local_addr()?;
        let shared = Arc::new(Shared {
            state: Mutex::new(State::seeded()),
            faults: Mutex::default(),
            log: Mutex::default(),
            anubis_difficulty: Mutex::default(),
            stopped: AtomicBool::new(false),
        });
        let accepting = shared.clone();
        thread::spawn(move || {
            for stream in listener.incoming() {
                if accepting.stopped.load(Ordering::SeqCst) {
                    return;
                }
                let Ok(stream) = stream else { continue };
                let shared = accepting.clone();
                thread::spawn(move || serve(stream, &shared, address));
            }
        });
        Ok(Self { address, shared })
    }

    pub fn base_url(&self) -> String {
        format!("http://{}", self.address)
    }

    pub fn inject(&self, fault: Fault) {
        lock(&self.shared.faults).push(fault);
    }

    pub fn clear_faults(&self) {
        lock(&self.shared.faults).clear();
    }

    /// Puts a proof-of-work challenge in front of every page until it is
    /// passed.
    pub fn require_anubis(&self, difficulty: usize) {
        *lock(&self.shared.anubis_difficulty) = Some(difficulty);
    }

    /// Every request served so far, oldest first.
    pub fn requests(&self) -> Vec<Request> {
        lock(&self.shared.log).clone()
    }

    pub fn requests_to(&self, path_contains: &str) -> Vec<Request> {
        self.requests()
            .into_iter()
            .filter(|request| request.path.contains(path_contains))
            .collect()
    }

    pub fn with_state<T>(&self, read: impl FnOnce(&mut State) -> T) -> T {
        read(&mut lock(&self.shared.state))
    }
}

impl Drop for FakeProvider {
    fn drop(&mut self) {
        self.shared.stopped.store(true, Ordering::SeqCst);
        // Wakes the accept loop so it can see the flag.
        let _ = TcpStream::connect(self.address);
    }
}

fn serve(mut stream: TcpStream, shared: &Shared, address: SocketAddr) {
    let Ok(Some(request)) = read_request(&mut stream) else {
        return;
    };
    // As the client reached it: an emulator comes through its own alias for this
    // machine, and an address it could not reach would break every link on a page.
    let base = match &request.host {
        Some(host) => format!("http://{host}"),
        None => format!("http://{address}"),
    };
    if let Some(response) = routes::control(&request, shared) {
        let _ = response.write_to(&mut stream);
        return;
    }
    lock(&shared.log).push(request.clone());

    let response = match take_fault(shared, &request.path) {
        Some(FaultKind::Drop) => return,
        Some(FaultKind::LoseReply) => {
            routes::respond(&request, shared, &base);
            return;
        }
        Some(FaultKind::Delay(wait)) => {
            thread::sleep(wait);
            routes::respond(&request, shared, &base)
        }
        Some(FaultKind::Status(status)) => Response::new(status, "text/html", ""),
        Some(FaultKind::Garbage) => Response::html("%%% not the page you asked for %%%"),
        Some(FaultKind::Empty) => Response::html(""),
        Some(FaultKind::LimitRefusal) => Response::new(
            404,
            "text/html",
            "<html><head><title>404 Not Found</title></head><body><center><h1>404 Not Found</h1></center><hr><center>nginx</center></body></html>",
        ),
        Some(FaultKind::SessionExpired) => {
            lock(&shared.state).sessions_valid = false;
            routes::respond(&request, shared, &base)
        }
        None => routes::respond(&request, shared, &base),
    };
    let _ = response.write_to(&mut stream);
}

fn take_fault(shared: &Shared, path: &str) -> Option<FaultKind> {
    let mut faults = lock(&shared.faults);
    let at = faults
        .iter()
        .position(|fault| path.contains(&fault.path_contains))?;
    let kind = faults[at].kind.clone();
    match faults[at].remaining {
        Some(1) => {
            faults.remove(at);
        }
        Some(left) => faults[at].remaining = Some(left - 1),
        None => {}
    }
    Some(kind)
}
