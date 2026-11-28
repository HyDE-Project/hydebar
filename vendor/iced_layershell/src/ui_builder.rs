//! UI building utilities for `iced_layershell`.
//!
//! Provides functions to build and rebuild iced user interfaces
//! for layer shell surfaces.

use std::collections::HashMap;

use iced_core::Theme;
use iced_runtime::user_interface::UserInterface;

use crate::settings::SurfaceId;
use crate::surface_manager::IcedSurface;

type Element<'a, M> = iced_core::Element<'a, M, Theme, iced_renderer::Renderer>;

/// Build a single `UserInterface` for a surface.
pub(crate) fn build_single_ui<'a, State, Message: 'a>(
    view: &dyn for<'v> Fn(&'v State, SurfaceId) -> Element<'v, Message>,
    user_state: &'a State,
    id: SurfaceId,
    iced_surfaces: &mut HashMap<SurfaceId, IcedSurface>,
    renderer: &mut iced_renderer::Renderer,
) -> UserInterface<'a, Message, Theme, iced_renderer::Renderer> {
    let iced_s = iced_surfaces.get_mut(&id).unwrap();
    let cache = iced_s.cache.take().unwrap_or_default();
    iced_s.needs_redraw = true;
    let element = view(user_state, id);
    UserInterface::build(element, iced_s.viewport.logical_size(), cache, renderer)
}

/// Build a [`UserInterface`] for every registered surface.
pub(crate) fn build_user_interfaces<'a, State, Message: 'a>(
    view: &dyn for<'v> Fn(&'v State, SurfaceId) -> Element<'v, Message>,
    user_state: &'a State,
    iced_surfaces: &mut HashMap<SurfaceId, IcedSurface>,
    renderer: &mut iced_renderer::Renderer,
) -> HashMap<SurfaceId, UserInterface<'a, Message, Theme, iced_renderer::Renderer>> {
    let ids: Vec<SurfaceId> = iced_surfaces.keys().copied().collect();
    ids.into_iter()
        .map(|id| {
            (
                id,
                build_single_ui(view, user_state, id, iced_surfaces, renderer),
            )
        })
        .collect()
}
