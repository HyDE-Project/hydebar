use std::{sync::Arc, time::Duration};

use hydebar_proto::ports::hyprland::{
    HyprlandMonitorSelector, HyprlandPort, HyprlandWorkspaceEvent, HyprlandWorkspaceSelector,
    HyprlandWorkspaceSnapshot
};
use iced::{
    Element, Length, Padding, SurfaceId as Id, alignment,
    widget::{Row, button, container}
};
use itertools::Itertools;
use log::{debug, error};
use tokio::{task::JoinHandle, time::sleep};
use tokio_stream::StreamExt;

use super::{Module, ModuleError, OnModulePress};
use crate::{
    ModuleContext, ModuleEventSender,
    components::text::text,
    config::{
        Appearance, MODULE_VERTICAL_PADDING_EM, WORKSPACE_ACTIVE_MARGIN_EM,
        WORKSPACE_ACTIVE_PADDING_EM, WORKSPACE_GAP_EM, WORKSPACE_GLYPH_ADVANCE_EM,
        WORKSPACE_MIN_HEIGHT_EM, WORKSPACE_MIN_WIDTH_EM, WORKSPACE_PADDING_EM,
        WorkspaceVisibilityMode, WorkspacesModuleConfig
    },
    event_bus::ModuleEvent,
    outputs::Outputs,
    style::workspace_button_style
};

const WORKSPACE_EVENT_RETRY_DELAY: Duration = Duration::from_millis(500);

#[derive(Debug, Clone)]
pub struct Workspace {
    pub id:         i32,
    pub name:       String,
    pub monitor_id: Option<usize>, // index for color lookup; may be None
    pub monitor:    String,        // monitor name for fallback
    pub active:     bool,
    pub windows:    u16
}

fn get_workspaces(port: &dyn HyprlandPort, config: &WorkspacesModuleConfig) -> Vec<Workspace> {
    let snapshot = match port.workspace_snapshot() {
        Ok(snapshot) => snapshot,
        Err(err) => {
            error!("failed to retrieve workspace snapshot: {err}");
            return Vec::new();
        }
    };

    map_snapshot_to_workspaces(&snapshot, config)
}

/// Maps a compositor snapshot onto the indicators the bar draws.
///
/// A workspace is drawn active when its own monitor shows it, so every
/// monitor of a multi-head setup highlights its current workspace rather
/// than only the one holding the focus. A snapshot whose monitors report no
/// active workspace falls back to the single focused one, which is what a
/// minimal test double provides.
fn map_snapshot_to_workspaces(
    snapshot: &HyprlandWorkspaceSnapshot,
    config: &WorkspacesModuleConfig
) -> Vec<Workspace> {
    let active = snapshot.active_workspace_id;
    let monitors = &snapshot.monitors;

    // Deduplicate by ID to avoid duplicates from Hyprland.
    let workspaces: Vec<_> = snapshot.workspaces.iter().unique_by(|w| w.id).collect();

    // Preallocate result vector.
    let mut result: Vec<Workspace> = Vec::with_capacity(workspaces.len());

    let (special, normal): (Vec<_>, Vec<_>) = workspaces.into_iter().partition(|w| w.id < 0);

    // Map special workspaces.
    for w in special.iter() {
        result.push(Workspace {
            id:         w.id,
            name:       w
                .name
                .as_str()
                .split(':')
                .next_back()
                .map_or_else(String::new, ToOwned::to_owned),
            // Option<i128> -> Option<usize> with bounds check.
            monitor_id: w.monitor_id,
            monitor:    w.monitor_name.clone(),
            active:     monitors
                .iter()
                .any(|m| m.special_workspace_id == Some(w.id)),
            windows:    w.window_count
        });
    }

    let any_monitor_reports = monitors.iter().any(|m| m.active_workspace_id.is_some());

    // Map normal workspaces.
    for w in normal.iter() {
        let shown = if any_monitor_reports {
            monitors.iter().any(|m| m.active_workspace_id == Some(w.id))
        } else {
            Some(w.id) == active
        };

        result.push(Workspace {
            id:         w.id,
            name:       w.name.clone(),
            monitor_id: w.monitor_id,
            monitor:    w.monitor_name.clone(),
            active:     shown,
            windows:    w.window_count
        });
    }

    if !config.enable_workspace_filling || normal.is_empty() {
        result.sort_by_key(|w| w.id);
        return result;
    }

    // Synthesize "missing" workspaces [1..=max_id] for filling UI.
    let existing_ids = normal.iter().map(|w| w.id).collect::<Vec<_>>();
    let mut max_id = *existing_ids.iter().max().unwrap_or(&0);
    if let Some(max_workspaces) = config.max_workspaces
        && max_workspaces > max_id as u32
    {
        max_id = max_workspaces as i32;
    }

    let missing_ids: Vec<i32> = (1..=max_id)
        .filter(|id| !existing_ids.contains(id))
        .collect();

    result.reserve(missing_ids.len());

    for id in missing_ids {
        result.push(Workspace {
            id,
            name: id.to_string(),
            monitor_id: None,
            monitor: String::new(),
            active: false,
            windows: 0
        });
    }

    result.sort_by_key(|w| w.id);
    result
}

/// Returns the width the label box of a workspace indicator reserves.
///
/// The reference waybar theme never lets a workspace button shrink below the
/// minimum width its GTK stylesheet reserves for a button, so a short label
/// sits in a fixed box and the indicators keep the same rhythm whatever they
/// read. A label already wider than that minimum sizes itself instead, which
/// keeps a long special workspace name from being squeezed.
fn label_box_width(label: &str, glyph_advance: f32, min_width: f32) -> Length {
    let natural = glyph_advance * label.chars().count() as f32;

    if natural < min_width {
        Length::Fixed(min_width)
    } else {
        Length::Shrink
    }
}

pub struct Workspaces {
    hyprland:   Arc<dyn HyprlandPort>,
    workspaces: Vec<Workspace>,
    sender:     Option<ModuleEventSender<Message>>,
    runtime:    Option<tokio::runtime::Handle>,
    task:       Option<JoinHandle<()>>
}

impl Workspaces {
    pub fn new(hyprland: Arc<dyn HyprlandPort>, config: &WorkspacesModuleConfig) -> Self {
        let workspaces = get_workspaces(hyprland.as_ref(), config);

        Self {
            hyprland,
            workspaces,
            sender: None,
            runtime: None,
            task: None
        }
    }

    #[cfg(test)]
    pub(crate) fn items(&self) -> &[Workspace] {
        &self.workspaces
    }

    /// Runs a compositor dispatch off the thread the bar draws on.
    ///
    /// The port retries with a timeout when the compositor socket stalls;
    /// waiting that out on the update thread would freeze every module. A
    /// module that was never registered has no runtime and dispatches inline,
    /// which keeps tests synchronous.
    fn spawn_dispatch(
        &self,
        dispatch: impl FnOnce() -> Result<(), hydebar_proto::ports::hyprland::HyprlandError>
        + Send
        + 'static
    ) {
        match &self.runtime {
            Some(runtime) => {
                runtime.spawn_blocking(move || {
                    if let Err(err) = dispatch() {
                        error!("failed to dispatch workspace command: {err}");
                    }
                });
            }
            None => {
                if let Err(err) = dispatch() {
                    error!("failed to dispatch workspace command: {err}");
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    WorkspacesChanged(HyprlandWorkspaceSnapshot),
    ChangeWorkspace(i32),
    ToggleSpecialWorkspace(i32)
}

impl Workspaces {
    pub fn update(&mut self, message: Message, config: &WorkspacesModuleConfig) {
        match message {
            Message::WorkspacesChanged(snapshot) => {
                self.workspaces = map_snapshot_to_workspaces(&snapshot, config);
            }
            Message::ChangeWorkspace(id) => {
                if id > 0 {
                    let already_active = self.workspaces.iter().any(|w| w.active && w.id == id);
                    if !already_active {
                        debug!("changing workspace to: {id}");
                        let port = Arc::clone(&self.hyprland);
                        self.spawn_dispatch(move || {
                            port.change_workspace(HyprlandWorkspaceSelector::Id(id))
                        });
                    }
                }
            }
            Message::ToggleSpecialWorkspace(id) => {
                if let Some(special) = self.workspaces.iter().find(|w| w.id == id && w.id < 0) {
                    debug!("toggle special workspace: {id}");

                    let monitor_ident = match special.monitor_id {
                        Some(idx) => HyprlandMonitorSelector::Id(idx),
                        None => HyprlandMonitorSelector::Name(special.monitor.clone())
                    };
                    let name = special.name.clone();
                    let port = Arc::clone(&self.hyprland);

                    self.spawn_dispatch(move || {
                        port.focus_and_toggle_special_workspace(monitor_ident, &name)
                    });
                }
            }
        }
    }
}

/// Resolves a fresh snapshot off the async workers and publishes it.
///
/// The port call blocks on the compositor socket with retries, so it runs on
/// the blocking pool rather than on a runtime worker the other modules share.
async fn publish_snapshot(port: &Arc<dyn HyprlandPort>, sender: &ModuleEventSender<Message>) {
    let port = Arc::clone(port);

    match tokio::task::spawn_blocking(move || port.workspace_snapshot()).await {
        Ok(Ok(snapshot)) => {
            if let Err(err) = sender.try_send(Message::WorkspacesChanged(snapshot)) {
                error!("failed to publish workspace update: {err}");
            }
        }
        Ok(Err(err)) => error!("failed to retrieve workspace snapshot: {err}"),
        Err(err) => error!("workspace snapshot task failed: {err}")
    }
}

/// Swallows the rest of an event burst before the snapshot is taken.
///
/// A window being dragged across monitors lands as a handful of events within
/// a few milliseconds; one snapshot at the end of the burst shows the same
/// state as one per event, without the round-trips.
async fn drain_burst(
    stream: &mut hydebar_proto::ports::hyprland::HyprlandEventStream<HyprlandWorkspaceEvent>
) {
    const SETTLE: Duration = Duration::from_millis(25);

    while let Ok(Some(_)) = tokio::time::timeout(SETTLE, stream.next()).await {}
}

impl<M> Module<M> for Workspaces
where
    M: 'static + Clone + From<Message>
{
    type ViewData<'a> = (&'a Outputs, Id, &'a WorkspacesModuleConfig, &'a Appearance);
    type RegistrationData<'a> = &'a WorkspacesModuleConfig;

    fn register(
        &mut self,
        ctx: &ModuleContext,
        _config: Self::RegistrationData<'_>
    ) -> Result<(), ModuleError> {
        self.sender = Some(ctx.module_sender(ModuleEvent::Workspaces));
        self.runtime = Some(ctx.runtime_handle().clone());

        if let Some(handle) = self.task.take() {
            handle.abort();
        }

        if let Some(sender) = self.sender.clone() {
            let hyprland = Arc::clone(&self.hyprland);
            self.task = Some(ctx.runtime_handle().spawn(async move {
                publish_snapshot(&hyprland, &sender).await;

                loop {
                    match hyprland.workspace_events() {
                        Ok(mut stream) => {
                            while let Some(event) = stream.next().await {
                                match event {
                                    Ok(
                                        HyprlandWorkspaceEvent::Added
                                        | HyprlandWorkspaceEvent::Changed
                                        | HyprlandWorkspaceEvent::Removed
                                        | HyprlandWorkspaceEvent::Moved
                                        | HyprlandWorkspaceEvent::SpecialChanged
                                        | HyprlandWorkspaceEvent::SpecialRemoved
                                        | HyprlandWorkspaceEvent::WindowClosed
                                        | HyprlandWorkspaceEvent::WindowOpened
                                        | HyprlandWorkspaceEvent::WindowMoved
                                        | HyprlandWorkspaceEvent::ActiveMonitorChanged
                                    ) => {
                                        drain_burst(&mut stream).await;
                                        publish_snapshot(&hyprland, &sender).await;
                                    }
                                    Err(err) => {
                                        error!("workspace event stream error: {err}");
                                    }
                                }
                            }
                        }
                        Err(err) => {
                            error!("failed to start workspace event stream: {err}");
                        }
                    }

                    sleep(WORKSPACE_EVENT_RETRY_DELAY).await;
                }
            }));
        }

        Ok(())
    }

    /// Drops the compositor event stream once the module leaves the bar.
    ///
    /// The listener holds an open Hyprland socket and republishes on every
    /// window and workspace change; a layout without workspaces would repaint
    /// the bar for each of them and show nothing new.
    fn deregister(&mut self) {
        if let Some(task) = self.task.take() {
            task.abort();
        }

        self.sender = None;
    }

    fn view(
        &self,
        (outputs, id, config, appearance): Self::ViewData<'_>
    ) -> Option<(Element<'static, M>, Option<OnModulePress<M>>)> {
        let monitor_name = outputs.get_monitor_name(id);

        let radius = appearance.pill_radius();
        let font_size = appearance.font_size_px();
        let vertical_padding = appearance.spacing(MODULE_VERTICAL_PADDING_EM);
        let idle_padding = appearance.spacing(WORKSPACE_PADDING_EM);
        let active_padding = appearance.spacing(WORKSPACE_ACTIVE_PADDING_EM);
        let active_margin = appearance.spacing(WORKSPACE_ACTIVE_MARGIN_EM);
        let min_label_width = appearance.spacing(WORKSPACE_MIN_WIDTH_EM);
        let min_height = appearance.spacing(WORKSPACE_MIN_HEIGHT_EM);
        let glyph_advance = appearance.spacing(WORKSPACE_GLYPH_ADVANCE_EM);
        let workspace_colors = appearance.workspace_colors.as_slice();
        let special_workspace_colors = appearance.special_workspace_colors.as_deref();

        Some((
            Row::with_children(
                self.workspaces
                    .iter()
                    .filter_map(|w| {
                        let on_this_screen = monitor_name.is_none_or(|name| w.monitor == name);

                        if config.visibility_mode == WorkspaceVisibilityMode::All
                            || on_this_screen
                            || !outputs.has_name(&w.monitor)
                        {
                            let empty = w.windows == 0;
                            let monitor = w.monitor_id;

                            // Safe color lookup by monitor index; None means "no color".
                            let color = monitor.map(|m| {
                                if w.id > 0 {
                                    workspace_colors.get(m).copied()
                                } else {
                                    special_workspace_colors
                                        .unwrap_or(workspace_colors)
                                        .get(m)
                                        .copied()
                                }
                            });

                            let w_id = w.id;
                            let w_name = w.name.clone();
                            let w_active = w.active;

                            let side_padding = if w_active {
                                active_padding
                            } else {
                                idle_padding
                            };

                            let label = if w_id < 0 { w_name } else { w_id.to_string() };
                            let label_width =
                                label_box_width(&label, glyph_advance, min_label_width);

                            let indicator = button(
                                container(text(label).size(font_size))
                                    .width(label_width)
                                    .align_x(alignment::Horizontal::Center)
                                    .align_y(alignment::Vertical::Center)
                            )
                            .style(workspace_button_style(empty, w_active, radius, color))
                            .padding([vertical_padding, side_padding])
                            .on_press(if w_id > 0 {
                                Message::ChangeWorkspace(w_id)
                            } else {
                                Message::ToggleSpecialWorkspace(w_id)
                            })
                            .width(Length::Shrink)
                            .height(Length::Fixed(min_height));

                            Some(if w_active {
                                container(indicator)
                                    .padding(
                                        Padding::ZERO.left(active_margin).right(active_margin)
                                    )
                                    .into()
                            } else {
                                Element::from(indicator)
                            })
                        } else {
                            None
                        }
                    })
                    .map(|elem: Element<'_, Message>| elem.map(M::from))
                    .collect::<Vec<Element<'_, M>>>()
            )
            .spacing(appearance.spacing(WORKSPACE_GAP_EM))
            .into(),
            None
        ))
    }

    // Background updates are delivered via the shared module event sender.
}

#[cfg(test)]
mod tests {
    use hydebar_proto::config::WorkspacesModuleConfig;

    use super::*;
    use crate::test_utils::MockHyprlandPort;

    #[test]
    fn initializes_from_port_snapshot() {
        let port = Arc::new(MockHyprlandPort::default());
        let port_trait: Arc<dyn HyprlandPort> = port.clone();
        let config = WorkspacesModuleConfig::default();

        let module = Workspaces::new(port_trait, &config);

        assert!(!module.items().is_empty());
    }

    #[test]
    fn change_workspace_dispatches_via_port() {
        let port = Arc::new(MockHyprlandPort::default());
        let port_trait: Arc<dyn HyprlandPort> = port.clone();
        let config = WorkspacesModuleConfig::default();

        let mut module = Workspaces::new(port_trait, &config);
        module.update(Message::ChangeWorkspace(2), &config);

        assert_eq!(port.workspace_calls(), 1);
    }

    #[test]
    fn short_labels_fill_the_minimum_box() {
        // The reference theme resolves to a 6px glyph advance and a 16px minimum.
        assert_eq!(label_box_width("1", 6.0, 16.0), Length::Fixed(16.0));
        assert_eq!(label_box_width("10", 6.0, 16.0), Length::Fixed(16.0));
        assert_eq!(label_box_width("100", 6.0, 16.0), Length::Shrink);
        assert_eq!(label_box_width("scratch", 6.0, 16.0), Length::Shrink);
    }
}
