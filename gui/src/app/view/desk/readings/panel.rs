//! The plain data a reading is made of: the panel, its drawings and the
//! helper that fills its rows.
//!
//! The rooms of [`super`] all answer in these types, and the drawing beside
//! them only places what was decided here. The data stands below the rooms so
//! that every room leans on it directly, and nothing leans back up through
//! the module root.

/// One window of a screen, as a share of that screen.
///
/// Kept as shares rather than pixels so a miniature can be drawn at whatever
/// size the column has room for, and so a reading taken on a four megapixel
/// screen draws the same on a laptop panel.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frame {
    /// Left edge, as a share of the screen's width.
    pub x:        f32,
    /// Top edge, as a share of the screen's height.
    pub y:        f32,
    /// Width, as a share of the screen's width.
    pub width:    f32,
    /// Height, as a share of the screen's height.
    pub height:   f32,
    /// Whether this is the window the keyboard is talking to.
    pub focused:  bool,
    /// Whether the window floats over the layout instead of tiling into it.
    pub floating: bool
}

/// One workspace as a miniature of the screen it would fill.
#[derive(Debug, Clone, PartialEq)]
pub struct Miniature {
    /// What the compositor calls it, which is what the user calls it.
    pub name:    String,
    /// Whether it is the one on screen.
    pub active:  bool,
    /// Whether something on it asked for attention.
    pub urgent:  bool,
    /// The windows standing on it, in the compositor's own order.
    pub windows: Vec<Frame>
}

/// A drawing a block carries above its lines.
///
/// Most blocks are a table and nothing else, because most readings are
/// numbers. Two are not: a wallpaper is a picture, and a set of workspaces is
/// a shape — a room with things standing in it — and both say more drawn than
/// they ever could spelled out.
#[derive(Debug, Clone, PartialEq)]
pub enum Figure {
    /// A picture the desktop already keeps.
    Picture(iced::widget::image::Handle),
    /// The workspaces of a screen, each with the windows standing on it.
    Overview {
        /// The workspaces themselves, in the compositor's own order.
        rooms:  Vec<Miniature>,
        /// The wallpaper they stand on, when the bar has read it.
        ///
        /// A room is what is in it and what it stands on: window shapes over
        /// bare colour say how the workspace is laid out, and the same shapes
        /// over the wallpaper say which desktop it is laid out on — which is
        /// what makes it a preview of the workspace rather than a diagram of
        /// it.
        ground: Option<iced::widget::image::Handle>
    },
    /// A row of pictures with the one in force big in the middle of it.
    Accordion(hydebar_core::modules::desk::looks::Reel),
    /// What pressing a module leads to: every choice of its own menu, as the
    /// glyph it is drawn with and the name it is called by.
    Choices(Vec<(String, String)>),
    /// The last few minutes of one reading, oldest first.
    Trace {
        /// The readings themselves.
        readings: Vec<f32>,
        /// What the top of the drawing stands for.
        ceiling:  f32
    }
}

/// One block of the canvas: a heading and the lines under it.
///
/// The heading is borrowed where the crate wrote it down and owned where the
/// user did: every block but one is named here, and a module the user wrote
/// carries the name their configuration gave it.
#[derive(Debug, Clone, PartialEq)]
pub struct Panel {
    /// Heading, drawn in the small capitals the columns are ruled by.
    pub title:  std::borrow::Cow<'static, str>,
    /// Label and value of every line, in drawing order.
    pub rows:   Vec<(String, String)>,
    /// What is drawn between the rule and the lines, on the blocks that carry
    /// one.
    pub figure: Option<Figure>
}

impl Panel {
    /// A panel with rows, or nothing at all when the source reported none.
    pub(super) fn of(
        title: impl Into<std::borrow::Cow<'static, str>>,
        rows: Vec<(String, String)>
    ) -> Option<Self> {
        (!rows.is_empty()).then_some(Self {
            title: title.into(),
            rows,
            figure: None
        })
    }

    /// A panel that carries a drawing, whether or not it carries rows.
    ///
    /// A drawing is a reading in its own right: a screen with one window on it
    /// has a miniature worth showing and next to nothing worth spelling out.
    pub(super) fn drawn(
        title: impl Into<std::borrow::Cow<'static, str>>,
        rows: Vec<(String, String)>,
        figure: Figure
    ) -> Self {
        Self {
            title: title.into(),
            rows,
            figure: Some(figure)
        }
    }
}

/// Pushes a row when the source answered for it.
pub(super) fn push(rows: &mut Vec<(String, String)>, label: &str, value: Option<String>) {
    if let Some(value) = value {
        rows.push((label.to_owned(), value));
    }
}
