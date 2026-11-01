//! Reports the main loop being kept from running, so a headless run can fail
//! on work that belongs on another thread.

use relm4::gtk::glib;
use std::cell::Cell;
use std::time::{Duration, Instant};

const TICK: Duration = Duration::from_millis(10);

/// Long enough to pass over a frame's layout in an unoptimized build, short
/// enough that a blocking file or network call shows.
const STALL: Duration = Duration::from_millis(300);

pub fn start() {
    let last_tick = Cell::new(Instant::now());
    glib::timeout_add_local(TICK, move || {
        let now = Instant::now();
        let waited = now.duration_since(last_tick.replace(now));
        if waited > STALL {
            log::warn!("main loop stalled for {}ms", waited.as_millis());
        }
        glib::ControlFlow::Continue
    });
}
