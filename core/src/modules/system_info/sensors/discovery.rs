//! Rebuilding the sensor set out of what the kernel publishes right now.

use std::{path::Path, time::Instant};

use super::{
    Gpu, HardwareSensors, Input,
    drm::{self, Card},
    hwmon, selection,
    selection::{ChipView, select_cpu, select_gpu},
    utility::{self, Feed, ProcessRunner, UTILITY_INTERVAL}
};

impl HardwareSensors {
    /// Rebuilds the sensor set out of what the kernel publishes right now.
    pub(super) fn discover(&mut self) {
        let chips = hwmon::scan(&self.hwmon_root);
        let cards = drm::scan(&self.drm_root);
        let labels: Vec<Vec<&str>> = chips
            .iter()
            .map(|chip| {
                chip.inputs
                    .iter()
                    .map(|input| input.label.as_str())
                    .collect()
            })
            .collect();
        let views: Vec<ChipView<'_>> = chips
            .iter()
            .enumerate()
            .map(|(index, chip)| ChipView {
                chip:   chip.name.as_str(),
                inputs: labels[index].as_slice(),
                facts:  chip.facts
            })
            .collect();

        let cpu = select_cpu(&views).map(|pick| Input {
            label: chips[pick.chip].inputs[pick.input].label.clone(),
            chip:  chips[pick.chip].name.clone(),
            path:  chips[pick.chip].inputs[pick.input].path.clone()
        });

        let gpu = select_gpu(&views, self.preferred_gpu.as_deref())
            .map(|pick| {
                let chip = &chips[pick.chip];
                let input = &chip.inputs[pick.input];

                Gpu {
                    name:      chip.name.clone(),
                    vendor:    pick.vendor,
                    placement: pick.placement,
                    input:     Some(Input {
                        label: input.label.clone(),
                        chip:  chip.name.clone(),
                        path:  input.path.clone()
                    }),
                    card:      pair_card(&cards, chip.device.as_deref())
                }
            })
            .or_else(|| card_only_gpu(&cards));

        self.cpu = cpu;
        self.gpu = gpu;
        self.utility = self.utility_feed();
        self.discovered_at = Some(Instant::now());
    }

    /// Starts, keeps or stops the vendor utility behind the selected
    /// device.
    ///
    /// It is started only where the kernel publishes neither a temperature
    /// nor a load for the device, and only when the program is
    /// installed, so a machine the kernel covers never spawns a
    /// process.
    fn utility_feed(&mut self) -> Option<Feed> {
        let gpu = self.gpu.as_ref()?;
        let covered =
            gpu.input.is_some() && gpu.card.as_ref().is_some_and(|card| card.busy.is_some());

        if covered {
            return None;
        }

        if let Some(feed) = self.utility.take()
            && feed.vendor() == gpu.vendor
        {
            return Some(feed);
        }

        let utility = utility::for_vendor(gpu.vendor)?;
        utility::on_path(utility.program)?;

        let card = gpu.card.clone();

        Some(Feed::spawn(
            utility,
            ProcessRunner,
            move || card.as_ref().is_none_or(|card| !card.is_asleep()),
            UTILITY_INTERVAL
        ))
    }
}

/// Card the kernel links to the same device as the chosen chip.
fn pair_card(cards: &[Card], device: Option<&Path>) -> Option<Card> {
    let device = device?;

    cards
        .iter()
        .find(|card| card.device.as_deref() == Some(device))
        .cloned()
}

/// Graphics device known from the rendering subsystem alone.
///
/// A driver that registers no monitoring chip still publishes a device,
/// which is what a machine reports while its vendor module lacks
/// monitoring support or while the reading comes from a vendor utility
/// instead.
fn card_only_gpu(cards: &[Card]) -> Option<Gpu> {
    let card = cards
        .iter()
        .filter(|card| card.vendor.is_some())
        .min_by_key(|card| (u8::from(card.busy.is_none()), card.driver.clone()))?;
    let vendor = card.vendor?;
    let facts = selection::ChipFacts::from_address(
        card.device
            .as_deref()
            .and_then(Path::file_name)
            .and_then(|address| address.to_str())
    );

    Some(Gpu {
        name: card
            .driver
            .clone()
            .unwrap_or_else(|| vendor.as_str().to_owned()),
        vendor,
        placement: selection::placement(vendor, facts),
        input: None,
        card: Some(card.clone())
    })
}
