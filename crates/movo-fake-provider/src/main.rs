//! Runs the fake provider until killed. The address comes from the first
//! argument, the URL it serves on goes to stdout.

use movo_fake_provider::FakeProvider;
use std::io::Write;

fn main() {
    let address = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:0".to_string());
    let provider = FakeProvider::bind(&address).unwrap_or_else(|error| {
        eprintln!("cannot listen on {address}: {error}");
        std::process::exit(1);
    });
    println!("{}", provider.base_url());
    let _ = std::io::stdout().flush();
    loop {
        std::thread::park();
    }
}
