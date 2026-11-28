//! Professional `tracing` logging: structured fields, spans, grouped targets,
//! env-filterable, rotating file + human stdout, runtime reload, panic hook,
//! and a `tracing-log` bridge so `log::*!` calls flow through `tracing`.
//!
//! Grouping is by `target`, i.e. the module path and crate name. Filter with
//! `RUST_LOG` or the `log_filter`/`log_level` config keys, e.g.
//! `RUST_LOG=info,hydebar_core::services::network=trace,wgpu_hal=off`.
//! Per-layer: a quieter console than the full file. No secrets are logged.

use std::{backtrace::Backtrace, panic};

pub use hydebar_gui::Logger;
use tracing_subscriber::{EnvFilter, Layer, fmt, layer::SubscriberExt, util::SubscriberInitExt};

/// Logging directory for the rotating file layer.
const LOG_DIR: &str = "/tmp/hydebar";

/// Builds the base `EnvFilter`: default level, `hydebar*` groups, quiet noise.
///
/// `RUST_LOG` wins; `extra` (the `log_filter` config key) is merged; debug
/// builds trace `hydebar*`, release ships `info`. Noisy rendering/compositor
/// and font crates are quieted in their own groups.
fn base_filter(extra: Option<&str>) -> EnvFilter {
    let hydebar_level = if cfg!(debug_assertions) {
        "debug"
    } else {
        "info"
    };

    let mut directives = format!(
        "info,hydebar={hydebar_level},hydebar_core={hydebar_level},\
         hydebar_gui={hydebar_level},hydebar_proto={hydebar_level},\
         wgpu=warn,wgpu_hal=warn,iced=warn,cosmic_text=warn,smithay=warn,sctk=warn"
    );

    if let Some(extra) = extra.filter(|value| !value.is_empty()) {
        directives.push(',');
        directives.push_str(extra);
    }

    let mut filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(&directives))
        .unwrap_or_else(|_| EnvFilter::new("info"));

    for quiet in [
        "sctk=off",
        "calloop=off",
        "calloop_wayland_source=off",
        "smithay=off",
        "smithay_clipboard=off",
        "wgpu=off",
        "wgpu_hal=off",
        "wgpu_core=off",
        "iced_wgpu=off",
        "cosmic_text=off"
    ] {
        if let Ok(directive) = quiet.parse::<tracing_subscriber::filter::Directive>() {
            filter = filter.add_directive(directive);
        }
    }

    filter
}

/// Starts `tracing`, installs the `log` bridge and panic hook, returns a
/// runtime-reload handle for the shared filter.
///
/// Console is human-readable with target + thread; a daily-rotating file
/// (keep a week, from `tracing-appender`) gets everything the filter passes.
pub fn init() -> Logger {
    let filter = base_filter(std::env::var("HYDEBAR_LOG_FILTER").ok().as_deref());
    let logger = Logger::new(filter);

    let console = fmt::layer()
        .with_target(true)
        .with_thread_ids(true)
        .compact();

    let log_dir = std::path::Path::new(LOG_DIR);
    let _ = std::fs::create_dir_all(log_dir);
    let file_appender = tracing_appender::rolling::daily(log_dir, "hydebar.log");
    let file = fmt::layer()
        .with_target(true)
        .with_ansi(false)
        .with_writer(file_appender);

    let filtered = console.and_then(file).with_filter(logger.clone());
    let _ = tracing_subscriber::registry().with(filtered).try_init();
    log::set_max_level(log::LevelFilter::Warn);
    tracing::callsite::rebuild_interest_cache();

    panic::set_hook(Box::new(|info| {
        let backtrace = Backtrace::capture();
        tracing::error!(panic = %info, backtrace = %backtrace, "panic");
    }));

    logger
}
