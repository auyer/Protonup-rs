//! Pure keyboard/gamepad-to-message mapping, pane focus grids and focus
//! movement rules. Everything in here is side-effect free so it can be unit
//! tested without an iced runtime.

use gilrs::Button;

use iced::keyboard::{self, Key};

use crate::message::{Focusable, Message, NavAction, Pane};
use crate::state::ProtonupGui;
use crate::views::{
    architecture_selection, confirm_reinstall, custom_location, download_progress,
    manage_installations, quick_update, selection_flow, sidebar, tool_selection, version_selection,
};

const STICK_DEADZONE: f32 = 0.5;

/// Maps an ignored iced keyboard event to a navigation message.
///
/// Returns `None` for events that should not drive navigation (repeats of
/// activation keys, unknown keys, …).
pub(crate) fn keyboard_to_message(event: &keyboard::Event) -> Option<Message> {
    let keyboard::Event::KeyPressed {
        key,
        modified_key,
        modifiers,
        repeat,
        ..
    } = event
    else {
        return None;
    };

    let named = match key {
        Key::Named(named) => *named,
        // `?` is layout-dependent, so it is matched on the modified character.
        Key::Character(_) => {
            if !repeat && modified_key.as_ref() == Key::Character("?") {
                return Some(Message::ToggleKeyboardShortcuts);
            }
            return None;
        }
        Key::Unidentified => return None,
    };

    let action = match named {
        keyboard::key::Named::Tab => {
            if modifiers.shift() {
                NavAction::FocusPreviousPane
            } else {
                NavAction::FocusNextPane
            }
        }
        keyboard::key::Named::ArrowUp => NavAction::MoveUp,
        keyboard::key::Named::ArrowDown => NavAction::MoveDown,
        keyboard::key::Named::ArrowLeft => NavAction::MoveLeft,
        keyboard::key::Named::ArrowRight => NavAction::MoveRight,
        keyboard::key::Named::Space if !repeat => NavAction::Select,
        keyboard::key::Named::Enter if !repeat => NavAction::Primary,
        keyboard::key::Named::Escape if !repeat => return Some(Message::CloseShortcuts),
        _ => return None,
    };

    Some(Message::Nav(action))
}

/// Maps a just-pressed gamepad button to a navigation message.
pub(crate) fn gamepad_to_message(button: Button) -> Option<Message> {
    let action = match button {
        Button::RightTrigger | Button::RightTrigger2 => NavAction::FocusNextPane,
        Button::LeftTrigger | Button::LeftTrigger2 => NavAction::FocusPreviousPane,
        Button::DPadUp => NavAction::MoveUp,
        Button::DPadDown => NavAction::MoveDown,
        Button::DPadLeft => NavAction::MoveLeft,
        Button::DPadRight => NavAction::MoveRight,
        Button::South => NavAction::Select,
        Button::Start => NavAction::Primary,
        Button::East => return Some(Message::CloseShortcuts),
        Button::North => return Some(Message::ToggleControllerShortcuts),
        _ => return None,
    };

    Some(Message::Nav(action))
}

/// The direction the left stick is currently pointing (dominant axis only).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StickDirection {
    Up,
    Down,
    Left,
    Right,
}

impl StickDirection {
    fn button(self) -> Button {
        match self {
            Self::Up => Button::DPadUp,
            Self::Down => Button::DPadDown,
            Self::Left => Button::DPadLeft,
            Self::Right => Button::DPadRight,
        }
    }
}

/// Cross-poll state for edge-triggering the left stick.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct StickState {
    last: (f32, f32),
    last_direction: Option<StickDirection>,
}

impl StickState {
    /// Feeds one stick sample in and emits at most one navigation message.
    ///
    /// The stick uses a deadzone and edge-triggered semantics: one step per
    /// threshold crossing, and the stick must return to the deadzone before
    /// the same direction fires again.
    pub fn sample(&mut self, stick: (f32, f32)) -> Option<Message> {
        let (x, y) = stick;
        let in_deadzone = x.abs() < STICK_DEADZONE && y.abs() < STICK_DEADZONE;

        let direction = if y.abs() >= STICK_DEADZONE && y.abs() >= x.abs() {
            if y < 0.0 {
                Some(StickDirection::Down)
            } else {
                Some(StickDirection::Up)
            }
        } else if x.abs() >= STICK_DEADZONE {
            if x < 0.0 {
                Some(StickDirection::Left)
            } else {
                Some(StickDirection::Right)
            }
        } else {
            None
        };

        if in_deadzone {
            self.last = (0.0, 0.0);
            self.last_direction = None;
            return None;
        }

        let message = (direction != self.last_direction)
            .then(|| direction.and_then(|d| gamepad_to_message(d.button())))
            .flatten();

        self.last = stick;
        self.last_direction = direction;
        message
    }
}

/// The focusable options of one pane, as a 2D grid of rows (each row is a
/// horizontal group of options in rendering order).
pub(crate) fn pane_grid(state: &ProtonupGui, pane: Pane) -> Vec<Vec<Focusable>> {
    match pane {
        Pane::Sidebar => sidebar::focusable_options(state),
        Pane::Main => main_grid(state),
        Pane::Actions => actions_grid(state),
    }
}

fn downloading(state: &ProtonupGui) -> bool {
    selection_flow::downloading(state)
}

fn main_grid(state: &ProtonupGui) -> Vec<Vec<Focusable>> {
    if downloading(state) {
        return download_progress::main_focusable_options(state);
    }

    match &state.mode {
        crate::message::GuiMode::QuickUpdate => quick_update::main_focusable_options(state),
        crate::message::GuiMode::DownloadForSteam
        | crate::message::GuiMode::DownloadForLutris
        | crate::message::GuiMode::DownloadForCustom
        | crate::message::GuiMode::CheckWhatsNew => {
            if state.mode == crate::message::GuiMode::DownloadForCustom
                && state.selection_step == crate::message::SelectionStep::Initial
            {
                custom_location::main_focusable_options(state)
            } else {
                main_selection_grid(state).unwrap_or_default()
            }
        }
        crate::message::GuiMode::ManageInstallations => {
            manage_installations::main_focusable_options(state)
        }
        _ => Vec::new(),
    }
}

fn main_selection_grid(state: &ProtonupGui) -> Option<Vec<Vec<Focusable>>> {
    match state.selection_step {
        crate::message::SelectionStep::SelectingTools => {
            Some(tool_selection::main_focusable_options(state))
        }
        crate::message::SelectionStep::SelectingVersions => {
            Some(version_selection::main_focusable_options(state))
        }
        crate::message::SelectionStep::SelectingArchitecture => {
            Some(architecture_selection::main_focusable_options(state))
        }
        crate::message::SelectionStep::ConfirmReinstall => {
            Some(confirm_reinstall::main_focusable_options(state))
        }
        _ => None,
    }
}

fn actions_grid(state: &ProtonupGui) -> Vec<Vec<Focusable>> {
    let actions: Vec<Focusable> = crate::views::main_content_actions(state)
        .into_iter()
        .filter_map(|action| {
            action
                .message
                .as_ref()
                .map(|message| Focusable::new(message.clone()))
        })
        .collect();

    if actions.is_empty() {
        Vec::new()
    } else {
        vec![actions]
    }
}

/// The view's primary "advance" action (Enter / Start), if any.
pub(crate) fn primary_action(state: &ProtonupGui) -> Option<Message> {
    if downloading(state) {
        return download_progress::primary_action(state);
    }

    match &state.mode {
        crate::message::GuiMode::QuickUpdate => quick_update::primary_action(state),
        crate::message::GuiMode::ManageInstallations => manage_installations::primary_action(state),
        crate::message::GuiMode::DownloadForCustom
            if state.selection_step == crate::message::SelectionStep::Initial =>
        {
            Some(Message::ToolSelectionConfirmed)
        }
        crate::message::GuiMode::DownloadForSteam
        | crate::message::GuiMode::DownloadForLutris
        | crate::message::GuiMode::DownloadForCustom
        | crate::message::GuiMode::CheckWhatsNew
            if state.selection_step == crate::message::SelectionStep::SelectingVersions =>
        {
            // Check What's New has no "advance" action; Enter falls back to
            // activating the focused version.
            (state.mode != crate::message::GuiMode::CheckWhatsNew)
                .then_some(Message::StartSelectedDownloads)
        }
        _ => None,
    }
}

/// Whether the given pane currently has any focusable option (i.e. is
/// relevant to the current view). Tabbing must never land on a hidden pane.
pub(crate) fn pane_is_active(state: &ProtonupGui, pane: Pane) -> bool {
    !pane_grid(state, pane).is_empty()
}

/// Applies a navigation action to the focus state, clamping the cursor into
/// the current pane grid. Returns the domain message to dispatch when the
/// action activates something.
pub(crate) fn apply_nav_action(state: &mut ProtonupGui, action: NavAction) -> Option<Message> {
    let focus = &mut state.focus;
    focus.engaged = true;

    match action {
        NavAction::FocusNextPane => {
            focus_pane_cycled(state, true);
        }
        NavAction::FocusPreviousPane => {
            focus_pane_cycled(state, false);
        }
        NavAction::MoveUp | NavAction::MoveDown | NavAction::MoveLeft | NavAction::MoveRight => {
            move_focus(state, action);
        }
        NavAction::Select => {
            return focused_option(state)
                .filter(|option| option.widget_focus.is_none())
                .map(|option| option.message);
        }
        NavAction::Primary => {
            return primary_action(state).or_else(|| focused_option(state).map(|o| o.message));
        }
    }

    None
}

/// Moves focus to the next (`true`) or previous (`false`) *active* pane,
/// skipping hidden panes until one with options is found.
pub(crate) fn focus_pane_cycled(state: &mut ProtonupGui, forward: bool) {
    let mut pane = if forward {
        state.focus.pane.next()
    } else {
        state.focus.pane.previous()
    };

    // Guard against a pane cycle with no active pane at all.
    for _ in 0..3 {
        if pane_is_active(state, pane) {
            break;
        }
        pane = if forward {
            pane.next()
        } else {
            pane.previous()
        };
    }

    state.focus.pane = pane;
    state.focus.row = 0;
    state.focus.col = 0;
}

/// Whether a `Select` on the current focus should focus a text input instead
/// of dispatching a message.
pub(crate) fn focused_widget_focus(state: &ProtonupGui) -> Option<iced::widget::Id> {
    focused_option(state)?.widget_focus
}

/// Moves the focus cursor within the current pane. Left/right navigation
/// falls back to pane cycling when there is no horizontal neighbor.
fn move_focus(state: &mut ProtonupGui, action: NavAction) {
    let pane = state.focus.pane;
    let grid = pane_grid(state, pane);
    let focus = &mut state.focus;

    // Clamps the current cursor into the grid (the grid may have shrunk
    // since the last move, e.g. when a list finished loading).
    let rows = grid.len();
    if rows == 0 {
        return;
    }
    let row = focus.row.min(rows - 1);
    let cols = grid[row].len().max(1);
    let col = focus.col.min(cols - 1);
    focus.row = row;
    focus.col = col;

    match action {
        NavAction::MoveUp => {
            if row > 0 {
                focus.row = row - 1;
            }
        }
        NavAction::MoveDown => {
            if row + 1 < rows {
                focus.row = row + 1;
            }
        }
        NavAction::MoveLeft => {
            if col > 0 {
                focus.col = col - 1;
            } else {
                focus_pane_cycled(state, false);
            }
        }
        NavAction::MoveRight => {
            if col + 1 < cols {
                focus.col = col + 1;
            } else {
                focus_pane_cycled(state, true);
            }
        }
        _ => {}
    }
}

/// The option under the focus cursor in the current pane.
pub(crate) fn focused_option(state: &ProtonupGui) -> Option<Focusable> {
    let (row, col) = (state.focus.row, state.focus.col);
    pane_grid(state, state.focus.pane)
        .into_iter()
        .nth(row)
        .and_then(|row| row.into_iter().nth(col))
}
