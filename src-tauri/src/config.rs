/// Default sync server URL.
/// Override at build time: SYNC_SERVER_URL=https://example.com cargo build
pub const SYNC_SERVER_URL: &str = match option_env!("SYNC_SERVER_URL") {
    Some(url) => url,
    None => "https://clipslot-production.up.railway.app",
};
