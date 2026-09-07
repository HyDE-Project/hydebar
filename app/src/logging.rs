//! The rotating file logger and the panic hook.

use std::{backtrace::Backtrace, panic};

use flexi_logger::{
    Age, Cleanup, Criterion, FileSpec, LogSpecBuilder, Logger, LoggerHandle, Naming
};
use log::error;

use crate::error::MainError;

/// Starts the logger writing under `/tmp/hydebar` and installs the panic hook.
///
/// The log rotates daily and keeps a week of files. The returned handle stays
/// with the caller so the level can be tightened once the configuration is
/// read.
///
/// The level is split the way a debug build and a release build differ: a
/// debug build (`cargo run` during development) traces the bar's own crates
/// down to `debug`, so every frame and every surface decision can be
/// followed; a release build ships `info` and above only, and the
/// development tracing is compiled in but never passes the filter.
pub fn init() -> Result<LoggerHandle, MainError> {
    let mut spec = LogSpecBuilder::new();
    spec.default(log::LevelFilter::Info);

    if cfg!(debug_assertions) {
        spec.module("hydebar", log::LevelFilter::Debug);
    }

    let logger = Logger::with(spec.build())
        .log_to_file(FileSpec::default().directory("/tmp/hydebar"))
        .duplicate_to_stdout(flexi_logger::Duplicate::All)
        .rotate(
            Criterion::Age(Age::Day),
            Naming::Timestamps,
            Cleanup::KeepLogFiles(7)
        );
    let logger = if cfg!(debug_assertions) {
        logger.duplicate_to_stdout(flexi_logger::Duplicate::All)
    } else {
        logger
    };
    let logger = logger.start()?;

    panic::set_hook(Box::new(|info| {
        let b = Backtrace::capture();
        error!("Panic: {info} \n {b}");
    }));

    Ok(logger)
}
