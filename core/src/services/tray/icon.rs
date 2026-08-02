//! Tray icon resolution: theme lookup, rasterisation, and trimming.

mod lookup;
mod raster;
mod theme;
mod trim;

pub use lookup::icon_from_name;
pub use raster::icon_from_pixmaps;
