//! Stable iced widget ids for the scrollable regions.
//!
//! Scrollables are given fixed ids so that the background gamepad stream and
//! the focus logic can drive them programmatically via `iced::widget::operation`.

use iced::widget::Id;

/// The sidebar's scrollable body. The pinned `Close` entry sits outside it and
/// stays visible at the bottom.
pub(crate) fn sidebar() -> Id {
    Id::new("scroll-sidebar")
}

/// The main content pane's primary scrollable (the list/grid of options).
pub(crate) fn main() -> Id {
    Id::new("scroll-main")
}

/// The nested right-hand scrollable shown alongside the main list in the
/// two-column views (changelog / installed-versions panes).
pub(crate) fn main_right() -> Id {
    Id::new("scroll-main-right")
}
