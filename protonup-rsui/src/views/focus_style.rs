//! Focus highlighting helpers shared by all views.
//!
//! A widget is "focused" when keyboard/controller navigation is engaged and
//! the focus cursor points at it. Views compute a [`Highlight`] per
//! interactive element so rendering stays in lockstep with
//! [`crate::input::pane_grid`].

use iced::widget::{button, checkbox, container, rule, text_input};
use iced::{Background, Border, Color, Element, Font, Length, Shadow, Theme, Vector};

use crate::message::{Message, Pane};
use crate::state::ProtonupGui;

/// Visible thickness of the focus ring, in logical
const FOCUS_RING_WIDTH: f32 = FOCUS_RING_VISIBLE * 2.0;

/// The ring thickness the user actually perceives, in logical pixels.
const FOCUS_RING_VISIBLE: f32 = 3.0;

/// Alpha of the focus fill drawn behind the focused option.
const FOCUS_FILL_ALPHA: f32 = 0.4;

/// Blur radius of the inner glow drawn inside the focus ring.
const FOCUS_GLOW_BLUR: f32 = FOCUS_RING_VISIBLE * 2.5;

/// Alpha of the inner glow.
const FOCUS_GLOW_ALPHA: f32 = 0.55;

/// Accent color of the focus ring, derived from the active theme.
fn focus_color(theme: &Theme) -> Color {
    theme.extended_palette().secondary.strong.color
}

/// Fill color of the focused option row.
fn focus_fill(theme: &Theme) -> Background {
    Background::Color(focus_color(theme).scale_alpha(FOCUS_FILL_ALPHA))
}

/// Inner glow drawn inside the focus ring: a soft, blurred accent shadow.
fn focus_glow(theme: &Theme) -> Shadow {
    Shadow {
        color: focus_color(theme).scale_alpha(FOCUS_GLOW_ALPHA),
        offset: Vector::ZERO,
        blur_radius: FOCUS_GLOW_BLUR,
    }
}

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

fn border_of(theme: &Theme, highlight: Highlight) -> Border {
    if highlight.is_focused() {
        Border {
            radius: 4.0.into(),
            width: FOCUS_RING_WIDTH,
            color: focus_color(theme),
        }
    } else {
        Border {
            radius: 4.0.into(),
            width: 0.0,
            color: Color::TRANSPARENT,
        }
    }
}

fn background_of(theme: &Theme, highlight: Highlight) -> Option<Background> {
    highlight.is_focused().then(|| focus_fill(theme))
}

/// Container style that draws the focus ring/fill when focused, and an
/// optional always-on background otherwise.
pub(crate) fn option_container_style(
    highlight: Highlight,
    base_background: Option<Background>,
) -> impl Fn(&Theme) -> container::Style + 'static {
    move |theme| {
        if highlight.is_focused() {
            container::Style {
                background: background_of(theme, highlight),
                border: border_of(theme, highlight),
                shadow: focus_glow(theme),
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
        Some(message) => focus_frame(
            button(content)
                .on_press(message)
                .padding(10)
                .width(Length::Fill)
                .style(option_button_style),
            highlight,
        ),
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
        Some(message) => focus_frame(
            button(content)
                .on_press(message)
                .padding(10)
                .width(Length::Fixed(width))
                .style(option_button_style),
            highlight,
        ),
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

/// Style for an option [`iced::widget::button`]: the standard button look.
pub(crate) fn option_button_style(theme: &Theme, status: button::Status) -> button::Style {
    let mut style = button::primary(theme, status);
    style.border = Border {
        radius: 4.0.into(),
        width: 1.0,
        color: theme.extended_palette().background.strong.color,
    };
    style
}

/// Wraps `content` in a focus ring when `highlight` is focused.
pub(crate) fn focus_frame<'a>(
    content: impl Into<Element<'a, Message>>,
    highlight: Highlight,
) -> Element<'a, Message> {
    container(content.into())
        .padding(if highlight.is_focused() {
            FOCUS_RING_VISIBLE
        } else {
            0.0
        })
        .style(option_container_style(highlight, None))
        .into()
}

/// Style for a [`iced::widget::checkbox`] that draws the focus ring on the box.
///
/// The checkbox draws its own chrome, so unlike [`focus_frame`] the ring has to
/// be an inline border here; [`FOCUS_RING_WIDTH`] already compensates for the
/// half-stroke clipping that iced applies to quad borders.
pub(crate) fn checkbox_style(
    highlight: Highlight,
) -> impl Fn(&Theme, checkbox::Status) -> checkbox::Style + 'static {
    move |theme, status| {
        let mut style = checkbox::primary(theme, status);
        if highlight.is_focused() {
            style.border = Border {
                radius: 4.0.into(),
                width: FOCUS_RING_WIDTH,
                color: focus_color(theme),
            };
        }
        style
    }
}

/// Style for a [`iced::widget::text_input`] that shows a focus ring.
///
/// Like [`checkbox_style`], this is an inline border with the width already
/// compensated for iced's half-stroke clipping.
pub(crate) fn text_input_style(
    highlight: Highlight,
) -> impl Fn(&Theme, text_input::Status) -> text_input::Style + 'static {
    move |theme, status| {
        let mut style = text_input::default(theme, status);
        if highlight.is_focused() {
            style.border = Border {
                radius: 4.0.into(),
                width: FOCUS_RING_WIDTH,
                color: focus_color(theme),
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
                width: FOCUS_RING_WIDTH,
                color: focus_color(theme),
            };
            style.shadow = focus_glow(theme);
        }
        style
    }
}

/// Style for a chrome panel (header, sidebar, footer).
pub(crate) fn panel_style() -> impl Fn(&Theme) -> container::Style + 'static {
    |theme| {
        let palette = theme.extended_palette();
        container::Style {
            background: Some(Background::Color(palette.background.weak.color)),
            border: Border {
                radius: 4.0.into(),
                width: 1.0,
                color: panel_border_color(theme),
            },
            ..container::Style::default()
        }
    }
}

/// Border color for a chrome panel.
pub(crate) fn panel_border_color(theme: &Theme) -> Color {
    let palette = theme.extended_palette();

    if palette.background.base.color == Color::WHITE {
        Color {
            a: 0.35,
            ..palette.background.base.text
        }
    } else {
        palette.background.strongest.color
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
                color: panel_border_color(theme),
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Relative luminance (WCAG) of a color with an opaque background behind it.
    fn luminance_on(color: Color, behind: Color) -> f32 {
        let a = color.a;
        let r = color.r * a + behind.r * (1.0 - a);
        let g = color.g * a + behind.g * (1.0 - a);
        let b = color.b * a + behind.b * (1.0 - a);

        fn channel(c: f32) -> f32 {
            if c <= 0.03928 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        }

        0.2126 * channel(r) + 0.7152 * channel(g) + 0.0722 * channel(b)
    }

    fn contrast_on(color: Color, behind: Color) -> f32 {
        let l1 = luminance_on(color, behind);
        let l2 = luminance_on(behind, behind);
        let (hi, lo) = if l1 > l2 { (l1, l2) } else { (l2, l1) };
        (hi + 0.05) / (lo + 0.05)
    }

    /// The panel border must be discernible against a light window background.
    ///
    /// iced's `background.strong*` shades are only ~15–20% off white, which
    /// made the header/sidebar/footer edges effectively invisible under
    /// gamescope light mode.
    #[test]
    fn panel_border_reads_in_light_theme() {
        let theme = Theme::Light;
        let palette = theme.extended_palette();
        assert_eq!(palette.background.base.color, Color::WHITE);

        let border = panel_border_color(&theme);
        let contrast = contrast_on(border, palette.background.base.color);

        assert!(
            contrast >= 1.3,
            "panel border contrast on light background was {contrast:.2}, expected >= 1.3"
        );

        // The old `background.strong` shade would not have passed this bar.
        let legacy = contrast_on(
            palette.background.strong.color,
            palette.background.base.color,
        );
        assert!(
            contrast > legacy,
            "new border ({contrast:.2}) should beat the old strong shade ({legacy:.2})"
        );
    }

    /// In dark themes keep iced's strongest background shade.
    #[test]
    fn panel_border_keeps_strongest_shade_in_dark_theme() {
        let theme = Theme::Dark;
        let palette = theme.extended_palette();

        assert_eq!(
            panel_border_color(&theme),
            palette.background.strongest.color
        );
    }

    /// `rounded_box` carries a radius but no border width, which is why the
    /// sidebar used to have no visible edge.
    #[test]
    fn panel_style_draws_a_visible_border() {
        let style = panel_style()(&Theme::Light);

        assert_eq!(style.border.width, 1.0);
        assert!(style.border.color.a > 0.0);
        assert!(style.background.is_some());
    }
}
