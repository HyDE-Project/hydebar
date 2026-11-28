use crate::settings::{Anchor, KeyboardInteractivity, Layer, LayerShellSettings, SurfaceId};

/// A rectangle within a surface's coordinate space, used for input region specification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InputRegionRect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

/// A command to modify the layer shell state.
#[derive(Debug, Clone)]
pub enum LayerShellCommand {
    NewSurface(SurfaceId, LayerShellSettings),
    DestroySurface(SurfaceId),
    SetAnchor(SurfaceId, Anchor),
    SetLayer(SurfaceId, Layer),
    SetExclusiveZone(SurfaceId, i32),
    SetKeyboardInteractivity(SurfaceId, KeyboardInteractivity),
    SetSize(SurfaceId, (u32, u32)),
    SetMargin(SurfaceId, (i32, i32, i32, i32)),
    /// Restrict pointer/touch input to specific rectangles. `None` resets to full surface.
    SetInputRegion(SurfaceId, Option<Vec<InputRegionRect>>),
}

/// A task that can be either a standard iced task or a layer shell command.
///
/// This wraps iced's `Task<M>` to also support layer shell commands,
/// giving an API identical to the pop-os iced fork without modifying iced.
pub enum Task<M> {
    /// A standard iced runtime task (async work, subscriptions, etc.)
    Iced(iced_runtime::Task<M>),
    /// A layer shell command (applied synchronously by the event loop).
    LayerShell(LayerShellCommand),
    /// Multiple tasks batched together.
    Batch(Vec<Task<M>>),
}

impl<M> Task<M> {
    /// A task that does nothing.
    #[must_use]
    pub fn none() -> Self {
        Self::Iced(iced_runtime::Task::none())
    }

    /// A task that immediately produces a message.
    pub fn done(value: M) -> Self
    where
        M: Send + 'static,
    {
        Self::Iced(iced_runtime::Task::done(value))
    }

    /// A task that runs an async operation and maps the result to a message.
    pub fn perform<A: Send + 'static>(
        future: impl std::future::Future<Output = A> + Send + 'static,
        f: impl FnOnce(A) -> M + Send + 'static,
    ) -> Self
    where
        M: Send + 'static,
    {
        Self::Iced(iced_runtime::Task::perform(future, f))
    }

    /// Batch multiple tasks together.
    #[allow(clippy::missing_panics_doc)] // unwrap is guarded by len() == 1
    pub fn batch(tasks: impl IntoIterator<Item = Self>) -> Self {
        let tasks: Vec<Self> = tasks.into_iter().collect();
        match tasks.len() {
            0 => Self::none(),
            1 => tasks.into_iter().next().unwrap(),
            _ => Self::Batch(tasks),
        }
    }

    /// Map the output of this task with the given function.
    pub fn map<N: Send + 'static>(self, f: impl Fn(M) -> N + Send + 'static + Clone) -> Task<N>
    where
        M: Send + 'static,
    {
        match self {
            Self::Iced(t) => Task::Iced(t.map(f)),
            Self::LayerShell(cmd) => Task::LayerShell(cmd),
            Self::Batch(tasks) => {
                Task::Batch(tasks.into_iter().map(|t| t.map(f.clone())).collect())
            }
        }
    }

    /// Make this task abortable. Returns the task and a handle to abort it.
    #[must_use]
    pub fn abortable(self) -> (Self, iced_runtime::task::Handle)
    where
        M: Send + 'static,
    {
        match self {
            Self::Iced(t) => {
                let (t, handle) = t.abortable();
                (Task::Iced(t), handle)
            }
            other => {
                // LayerShell commands and batches can't be aborted;
                // return a no-op handle
                let (_, handle) = iced_runtime::Task::<M>::none().abortable();
                (other, handle)
            }
        }
    }

    /// Chain another task after this one.
    #[must_use]
    pub fn chain(self, task: Self) -> Self
    where
        M: Send + 'static,
    {
        match (self, task) {
            (Self::Iced(a), Self::Iced(b)) => Task::Iced(a.chain(b)),
            (a, b) => Task::Batch(vec![a, b]),
        }
    }

    /// Discard the output of this task.
    #[must_use]
    pub fn discard<N>(self) -> Task<N>
    where
        M: Send + 'static,
        N: Send + 'static,
    {
        match self {
            Self::Iced(t) => Task::Iced(t.discard()),
            Self::LayerShell(cmd) => Task::LayerShell(cmd),
            Self::Batch(tasks) => Task::Batch(tasks.into_iter().map(Task::discard).collect()),
        }
    }
}

impl<M> From<iced_runtime::Task<M>> for Task<M> {
    fn from(task: iced_runtime::Task<M>) -> Self {
        Self::Iced(task)
    }
}

/// Create a new layer shell surface. Returns the assigned ID and a task.
#[must_use]
pub fn new_layer_surface<M>(settings: LayerShellSettings) -> (SurfaceId, Task<M>) {
    let id = SurfaceId::unique();
    (
        id,
        Task::LayerShell(LayerShellCommand::NewSurface(id, settings)),
    )
}

/// Destroy a layer shell surface.
#[must_use]
pub fn destroy_layer_surface<M>(id: SurfaceId) -> Task<M> {
    Task::LayerShell(LayerShellCommand::DestroySurface(id))
}

/// Change the anchor of a surface.
#[must_use]
pub fn set_anchor<M>(id: SurfaceId, anchor: Anchor) -> Task<M> {
    Task::LayerShell(LayerShellCommand::SetAnchor(id, anchor))
}

/// Change the layer of a surface.
#[must_use]
pub fn set_layer<M>(id: SurfaceId, layer: Layer) -> Task<M> {
    Task::LayerShell(LayerShellCommand::SetLayer(id, layer))
}

/// Change the exclusive zone of a surface.
#[must_use]
pub fn set_exclusive_zone<M>(id: SurfaceId, zone: i32) -> Task<M> {
    Task::LayerShell(LayerShellCommand::SetExclusiveZone(id, zone))
}

/// Change the keyboard interactivity of a surface.
#[must_use]
pub fn set_keyboard_interactivity<M>(id: SurfaceId, ki: KeyboardInteractivity) -> Task<M> {
    Task::LayerShell(LayerShellCommand::SetKeyboardInteractivity(id, ki))
}

/// Change the size of a surface.
#[must_use]
pub fn set_size<M>(id: SurfaceId, size: (u32, u32)) -> Task<M> {
    Task::LayerShell(LayerShellCommand::SetSize(id, size))
}

/// Change the margin of a surface.
#[must_use]
pub fn set_margin<M>(id: SurfaceId, margin: (i32, i32, i32, i32)) -> Task<M> {
    Task::LayerShell(LayerShellCommand::SetMargin(id, margin))
}

/// Set the input region of a surface.
///
/// Only the specified rectangles will accept pointer/touch input; the rest of
/// the surface becomes click-through. Pass `None` to reset to the default
/// (full surface accepts input).
#[must_use]
pub fn set_input_region<M>(id: SurfaceId, rects: Option<Vec<InputRegionRect>>) -> Task<M> {
    Task::LayerShell(LayerShellCommand::SetInputRegion(id, rects))
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- Task::batch ---

    #[test]
    fn batch_empty_is_none() {
        let task: Task<()> = Task::batch(vec![]);
        assert!(matches!(task, Task::Iced(_)));
    }

    #[test]
    fn batch_single_unwraps() {
        let id = SurfaceId::new(1);
        let task: Task<()> = Task::batch(vec![destroy_layer_surface(id)]);
        assert!(matches!(
            task,
            Task::LayerShell(LayerShellCommand::DestroySurface(_))
        ));
    }

    #[test]
    fn batch_multiple_creates_batch() {
        let id = SurfaceId::new(1);
        let task: Task<()> =
            Task::batch(vec![destroy_layer_surface(id), set_layer(id, Layer::Top)]);
        assert!(matches!(task, Task::Batch(v) if v.len() == 2));
    }

    // --- Task::map ---

    #[test]
    fn map_layer_shell_passes_through() {
        let id = SurfaceId::new(1);
        let task: Task<i32> = destroy_layer_surface(id);
        let mapped: Task<String> = task.map(|n: i32| n.to_string());
        assert!(matches!(
            mapped,
            Task::LayerShell(LayerShellCommand::DestroySurface(_))
        ));
    }

    // --- Task::chain ---

    #[test]
    fn chain_mixed_creates_batch() {
        let id = SurfaceId::new(1);
        let a: Task<()> = Task::none();
        let b: Task<()> = destroy_layer_surface(id);
        let chained = a.chain(b);
        assert!(matches!(chained, Task::Batch(v) if v.len() == 2));
    }

    // --- Task::discard ---

    #[test]
    fn discard_layer_shell_passes_through() {
        let id = SurfaceId::new(1);
        let task: Task<i32> = set_exclusive_zone(id, 40);
        let discarded: Task<String> = task.discard();
        assert!(matches!(
            discarded,
            Task::LayerShell(LayerShellCommand::SetExclusiveZone(_, 40))
        ));
    }

    // --- Task::from ---

    #[test]
    fn from_iced_task() {
        let task: Task<()> = Task::from(iced_runtime::Task::none());
        assert!(matches!(task, Task::Iced(_)));
    }

    // --- Free functions ---

    #[test]
    fn new_layer_surface_returns_unique_id() {
        let (id1, _): (SurfaceId, Task<()>) = new_layer_surface(LayerShellSettings::default());
        let (id2, _): (SurfaceId, Task<()>) = new_layer_surface(LayerShellSettings::default());
        assert_ne!(id1, id2);
    }

    #[test]
    fn destroy_layer_surface_creates_correct_command() {
        let id = SurfaceId::new(42);
        let task: Task<()> = destroy_layer_surface(id);
        assert!(
            matches!(task, Task::LayerShell(LayerShellCommand::DestroySurface(sid)) if sid == id)
        );
    }

    #[test]
    fn set_anchor_creates_correct_command() {
        let id = SurfaceId::new(1);
        let anchor = Anchor::TOP | Anchor::LEFT;
        let task: Task<()> = set_anchor(id, anchor);
        assert!(
            matches!(task, Task::LayerShell(LayerShellCommand::SetAnchor(_, a)) if a == anchor)
        );
    }

    #[test]
    fn set_size_creates_correct_command() {
        let id = SurfaceId::new(1);
        let task: Task<()> = set_size(id, (800, 600));
        assert!(matches!(
            task,
            Task::LayerShell(LayerShellCommand::SetSize(_, (800, 600)))
        ));
    }

    #[test]
    fn set_margin_creates_correct_command() {
        let id = SurfaceId::new(1);
        let task: Task<()> = set_margin(id, (10, 20, 30, 40));
        assert!(matches!(
            task,
            Task::LayerShell(LayerShellCommand::SetMargin(_, (10, 20, 30, 40)))
        ));
    }
}
