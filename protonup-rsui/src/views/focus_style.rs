//! Focus highlighting helpers shared by all views.
//!
//! A widget is "focused" when keyboard/controller navigation is engaged and
//! the focus cursor points at it. Views compute a [`Highlight`] per
//! interactive element so rendering stays in lockstep with
//! [`crate::input::pane_grid`].

use iced::widget::{button, checkbox, container, rule, text_input};
use iced::{Background, Border, Color, Element, Font, Length, Theme};

use crate::message::{Message, Pane};
use crate::state::ProtonupGui;

/// Accent color of the focus ring.
const FOCUS_COLOR: Color = Color::from_rgb(0.30, 0.62, 1.0);

/// Fill color of the focused option row.
const FOCUS_FILL: Color = Color::from_rgba(0.30, 0.62, 1.0, 0.18);

/// Which element of a pane should be drawn as focused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Highlight {
    /// This element is not focused.
    None,
    /// The focus cursor is on this element.
    Focused,
}

impl Highlight {
    pub(crate) fn is_focused(self) -> bool {
        matches!(self, Self::Focused)
    }
}

impl From<bool> for Highlight {
    fn from(focused: bool) -> Self {
        if focused { Self::Focused } else { Self::None }
    }
}

/// The focus cursor position, if focus highlighting is engaged for `pane`.
pub(crate) fn cursor(state: &ProtonupGui, pane: Pane) -> Option<(usize, usize)> {
    (state.focus.engaged && state.focus.pane == pane).then_some((state.focus.row, state.focus.col))
}

/// Whether the option at `(row, col)` inside `pane` is the focused one.
pub(crate) fn is_focused(state: &ProtonupGui, pane: Pane, row: usize, col: usize) -> bool {
    cursor(state, pane) == Some((row, col))
}

/// Whether the action-bar button at horizontal position `col` is focused.
pub(crate) fn action_focused(state: &ProtonupGui, col: usize) -> bool {
    is_focused(state, Pane::Actions, 0, col)
}

fn border_of(highlight: Highlight) -> Border {
    if highlight.is_focused() {
        Border {
            radius: 4.0.into(),
            width: 2.0,
            color: FOCUS_COLOR,
        }
    } else {
        Border {
            radius: 4.0.into(),
            width: 0.0,
            color: Color::TRANSPARENT,
        }
    }
}

fn background_of(highlight: Highlight) -> Option<Background> {
    highlight
        .is_focused()
        .then_some(Background::Color(FOCUS_FILL))
}

/// Container style that draws the focus ring/fill when focused, and an
/// optional always-on background otherwise.
pub(crate) fn option_container_style(
    highlight: Highlight,
    base_background: Option<Background>,
) -> impl Fn(&Theme) -> container::Style + 'static {
    move |_theme| {
        if highlight.is_focused() {
            container::Style {
                background: background_of(highlight),
                border: border_of(highlight),
                ..container::Style::default()
            }
        } else {
            container::Style {
                background: base_background,
                ..container::Style::default()
            }
        }
    }
}

/// A focusable, clickable option row.
///
/// Enabled options render as real [`iced::widget::button`]s so they keep the
/// standard button appearance; disabled options render as a dimmed, flat row
/// so option order stays stable for navigation.
pub(crate) fn option_row<'a>(
    content: impl Into<Element<'a, Message>>,
    highlight: Highlight,
    on_press: Option<Message>,
) -> Element<'a, Message> {
    let content: Element<'a, Message> = content.into();

    match on_press {
        Some(message) => button(content)
            .on_press(message)
            .padding(10)
            .width(Length::Fill)
            .style(option_button_style(highlight))
            .into(),
        None => container(content)
            .padding(10)
            .width(Length::Fill)
            .style(|theme: &Theme| container::Style {
                text_color: Some(theme.extended_palette().background.weak.text),
                ..container::Style::default()
            })
            .into(),
    }
}

/// A list-selection option: a full button look with a uniform width.
pub(crate) fn option_list_row<'a>(
    content: impl Into<Element<'a, Message>>,
    highlight: Highlight,
    on_press: Option<Message>,
    width: f32,
) -> Element<'a, Message> {
    let content: Element<'a, Message> = content.into();

    match on_press {
        Some(message) => button(content)
            .on_press(message)
            .padding(10)
            .width(Length::Fixed(width))
            .style(option_button_style(highlight))
            .into(),
        None => container(content)
            .padding(10)
            .width(Length::Fixed(width))
            .into(),
    }
}

/// A checkbox option row focused as a whole: the label is clickable and the
/// checkbox itself stays interactive.
///
/// Uses the same fixed `width` as list-selection options so checkbox rows line
/// up with them instead of stretching across the whole pane.
pub(crate) fn option_checkbox_row<'a>(
    is_checked: bool,
    label: &'a str,
    highlight: Highlight,
    toggle: Message,
    width: f32,
) -> Element<'a, Message> {
    let row = iced::widget::Row::new()
        .spacing(10)
        .align_y(iced::Center)
        .push(
            checkbox(is_checked)
                .on_toggle({
                    let toggle = toggle.clone();
                    move |_| toggle.clone()
                })
                .style(checkbox_style(highlight)),
        )
        .push(iced::widget::text(label).size(14));

    option_list_row(row, highlight, Some(toggle), width)
}

/// Style for an option [`iced::widget::button`]: the standard button look,
/// with a focus ring when the keyboard/controller cursor is on it.
pub(crate) fn option_button_style(
    highlight: Highlight,
) -> impl Fn(&Theme, button::Status) -> button::Style + 'static {
    move |theme, status| {
        let mut style = button::primary(theme, status);
        style.border = if highlight.is_focused() {
            Border {
                radius: 4.0.into(),
                width: 2.0,
                color: FOCUS_COLOR,
            }
        } else {
            Border {
                radius: 4.0.into(),
                width: 1.0,
                color: theme.extended_palette().background.strong.color,
            }
        };
        style
    }
}

/// Wraps `content` in a focus ring when `highlight` is focused.
///
/// Used for action buttons, where the ring is drawn by an outer container so
/// iced's button style is untouched.
pub(crate) fn focus_frame<'a>(
    content: impl Into<Element<'a, Message>>,
    highlight: Highlight,
) -> Element<'a, Message> {
    container(content.into())
        .padding(if highlight.is_focused() { 2 } else { 0 })
        .style(option_container_style(highlight, None))
        .into()
}

/// Style for a [`iced::widget::checkbox`] that draws the focus ring on the box.
pub(crate) fn checkbox_style(
    highlight: Highlight,
) -> impl Fn(&Theme, checkbox::Status) -> checkbox::Style + 'static {
    move |theme, status| {
        let mut style = checkbox::primary(theme, status);
        if highlight.is_focused() {
            style.border = Border {
                radius: 4.0.into(),
                width: 2.0,
                color: FOCUS_COLOR,
            };
        }
        style
    }
}

/// Style for a [`iced::widget::text_input`] that shows a focus ring.
pub(crate) fn text_input_style(
    highlight: Highlight,
) -> impl Fn(&Theme, text_input::Status) -> text_input::Style + 'static {
    move |theme, status| {
        let mut style = text_input::default(theme, status);
        if highlight.is_focused() {
            style.border = Border {
                radius: 4.0.into(),
                width: 2.0,
                color: FOCUS_COLOR,
            };
        }
        style
    }
}

/// Style for the action-bar container, drawing a focus fill when the cursor
/// is inside the action pane.
pub(crate) fn action_bar_style(
    highlight: Highlight,
) -> impl Fn(&Theme) -> container::Style + 'static {
    move |theme| {
        let mut style = container::rounded_box(theme);
        if highlight.is_focused() {
            style.border = Border {
                radius: 4.0.into(),
                width: 2.0,
                color: FOCUS_COLOR,
            };
        }
        style
    }
}

/// Style for the footer hint bar.
pub(crate) fn footer_style() -> impl Fn(&Theme) -> container::Style + 'static {
    |theme| {
        let palette = theme.extended_palette();
        container::Style {
            background: Some(Background::Color(palette.background.weak.color)),
            border: Border {
                radius: 4.0.into(),
                width: 1.0,
                color: palette.background.strong.color,
            },
            ..container::Style::default()
        }
    }
}

/// Monospaced font used for key hints.
pub(crate) fn hint_font() -> Font {
    Font::MONOSPACE
}

/// Builds a keycap-styled inline hint like `[TAB]`.
pub(crate) fn keycap<'a>(label: &'a str) -> Element<'a, Message> {
    container(iced::widget::text(label).size(11).font(hint_font()))
        .padding([1, 5])
        .style(|theme: &Theme| {
            let palette = theme.extended_palette();
            container::Style {
                background: Some(Background::Color(palette.background.strong.color)),
                border: Border {
                    radius: 3.0.into(),
                    width: 1.0,
                    color: palette.background.stronger.color,
                },
                ..container::Style::default()
            }
        })
        .into()
}

/// A vertical divider/rule helper re-exported for shortcut panels.
pub(crate) fn panel_rule() -> Element<'static, Message> {
    rule::horizontal(1).into()
}
