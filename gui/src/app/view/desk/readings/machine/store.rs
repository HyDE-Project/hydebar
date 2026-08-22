//! Where the machine keeps things, and what is crossing its link.

use hydebar_core::modules::system_info::{DiskData, SystemInfoData, gigabytes, used_of_total};

use super::super::{Panel, push};

/// The link: where it lands and what is crossing it.
pub fn network(data: &SystemInfoData) -> Option<Panel> {
    let network = data.network.as_ref()?;

    Panel::of(
        "network",
        vec![
            ("address".to_owned(), network.ip.clone()),
            (
                "down".to_owned(),
                format!("{} KB/s", network.download_speed)
            ),
            ("up".to_owned(), format!("{} KB/s", network.upload_speed)),
            ("taken in".to_owned(), bytes(network.received)),
            ("sent out".to_owned(), bytes(network.transmitted)),
        ]
    )
}

/// A count of bytes, said in the unit a person would use for it.
///
/// Traffic since a machine came up runs from kilobytes on a fresh boot to
/// tens of gigabytes a week later, and one unit cannot carry both.
fn bytes(count: u64) -> String {
    const STEPS: [(u64, &str); 4] = [
        (1024 * 1024 * 1024 * 1024, "TiB"),
        (1024 * 1024 * 1024, "GiB"),
        (1024 * 1024, "MiB"),
        (1024, "KiB")
    ];

    for (step, unit) in STEPS {
        if count >= step {
            #[expect(
                clippy::cast_precision_loss,
                reason = "a byte count is far below the precision limit of f64"
            )]
            let share = count as f64 / step as f64;

            return format!("{share:.1} {unit}");
        }
    }

    format!("{count} B")
}

/// Memory and swap, each in use against what there is.
pub fn memory(data: &SystemInfoData) -> Option<Panel> {
    let mut rows = vec![
        (
            "in use".to_owned(),
            format!(
                "{} ({}%)",
                used_of_total(data.memory_used, data.memory_total),
                data.memory_usage
            )
        ),
        (
            "available".to_owned(),
            format!(
                "{} GiB",
                gigabytes(data.memory_total.saturating_sub(data.memory_used))
            )
        ),
        (
            "cached".to_owned(),
            format!("{} GiB", gigabytes(data.memory_cached))
        ),
    ];

    if data.memory_swap_total > 0 {
        rows.push((
            "swap".to_owned(),
            format!(
                "{} ({}%)",
                used_of_total(data.memory_swap_used, data.memory_swap_total),
                data.memory_swap_usage
            )
        ));
        push(&mut rows, "backend", data.swap_backend.clone());
    }

    Panel::of("memory", rows)
}

/// Every filesystem, with what is left on it.
///
/// One line per filesystem rather than per mount point: a machine laid out
/// in subvolumes mounts the same filesystem a dozen times over — `/`,
/// `/home`, `/var/log` and the rest of them — and each of those mounts
/// reports the same bytes. Listing them all would fill the column with one
/// number repeated. The shortest mount stands for the filesystem, because
/// that is the one the user thinks of it by.
pub fn storage(data: &SystemInfoData) -> Option<Panel> {
    let mut filesystems: Vec<&DiskData> = Vec::new();

    for disk in &data.disks {
        match filesystems
            .iter_mut()
            .find(|seen| seen.used == disk.used && seen.total == disk.total)
        {
            Some(seen) => {
                if disk.mount.len() < seen.mount.len() {
                    *seen = disk;
                }
            }
            None => filesystems.push(disk)
        }
    }

    let mut rows: Vec<(String, String)> = filesystems
        .into_iter()
        .map(|disk| (disk.mount.clone(), used_of_total(disk.used, disk.total)))
        .collect();

    if let Some((read, written)) = data.disk_traffic {
        rows.push(("read".to_owned(), bytes(read)));
        rows.push(("written".to_owned(), bytes(written)));
    }

    Panel::of("storage", rows)
}
