//! The readouts the system information module can draw.

use serde::Deserialize;

/// Readout rendered by the memory indicators.
#[derive(Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MemoryFormat {
    /// Share of the total memory in use, for instance `50%`.
    #[default]
    Percentage,
    /// Amount of memory in use, for instance `7.8GB`.
    Bytes
}

/// A single readout rendered by the system information module.
#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum SystemIndicator {
    /// Share of the processor that is busy.
    Cpu,
    /// Share of the memory in use.
    Memory,
    /// Share of the swap in use.
    MemorySwap,
    /// Processor temperature.
    ///
    /// `Temperature` is the name earlier configurations used for it, and it
    /// keeps working now that the graphics has a reading of its own.
    #[serde(alias = "Temperature")]
    CpuTemperature,
    /// Graphics temperature.
    GpuTemperature,
    /// Share of the graphics processor that is busy.
    GpuUsage,
    /// Share of the named mount point in use.
    Disk(String),
    /// The address the machine answers on.
    IpAddress,
    /// How fast the link is receiving.
    DownloadSpeed,
    /// How fast the link is sending.
    UploadSpeed
}
