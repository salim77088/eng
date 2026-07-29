//! Logging helpers around the `log` crate. `init()` installs a
//! pretty-printing `env_logger` with sane defaults so any crate in
//! the workspace can just call `log::info!(...)`.

use std::sync::Once;

static INIT: Once = Once::new();

/// Initialize the global logger. Safe to call multiple times - subsequent
/// calls are no-ops.
pub fn init() {
    INIT.call_once(|| {
        let _ = env_logger::Builder::from_env(
            env_logger::Env::default().default_filter_or("lumina=info,wgpu=warn,naga=warn"),
        )
        .format_timestamp_millis()
        .try_init();
    });
}
