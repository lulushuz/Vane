#[cfg(target_os = "linux")]
pub mod router;
pub mod stats;
pub mod watcher;

pub use stats::get_total_network_bytes;
pub use watcher::spawn_network_watcher;
