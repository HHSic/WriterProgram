//! A stand-in drive for trying the app without an account.
//!
//! ```text
//! cargo run -p writer-sync --example stand_in -- dropbox 8765
//! ```
//!
//! Then start the app with `WRITER_DROPBOX_ENDPOINT=http://127.0.0.1:8765` and
//! `WRITER_SIGN_IN=fetch`, and a `drive-apps.json` in its settings folder with
//! any app id for that drive, e.g. `{"dropbox": {"clientId": "stand-in"}}`.
//! Signing in then goes through at once, and projects kept in step land in
//! the stand-in's memory until it stops.
//!
//! To try a drive filling up, set how full it is (bytes; `total: null` for
//! no limit, `readable: false` to refuse saying):
//!
//! ```text
//! curl -X POST http://127.0.0.1:8765/stand-in/room -d '{"total":41943040,"elsewhere":31457280,"readable":true}'
//! ```

#[path = "../tests/support/mod.rs"]
mod support;

use writer_sync::providers::Provider;

fn main() {
    let mut args = std::env::args().skip(1);
    let provider = match args.next().as_deref() {
        Some("google") => Provider::Google,
        Some("onedrive") => Provider::Onedrive,
        Some("dropbox") => Provider::Dropbox,
        _ => {
            eprintln!("usage: stand_in <google|onedrive|dropbox> [port]");
            std::process::exit(2);
        }
    };
    let port: u16 = args.next().and_then(|p| p.parse().ok()).unwrap_or(8765);
    let ends = support::stand_in_on(provider, port);
    println!(
        "{} stand-in at {}",
        provider.label(),
        ends.token.trim_end_matches("/token")
    );
    loop {
        std::thread::park();
    }
}
