mod input;
mod rendering;

use std::any::Any;

use crate::state::automation::{AutomationLaneId, AutomationTarget};
use crate::state::AppState;
use crate::ui::action_id::ActionId;
use crate::ui::layout_helpers::center_rect;
use crate::ui::{Rect, RenderBuf, Action, Color, InputEvent, Keymap, Pane, Style};

/// Focus area within the automation pane
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AutomationFocus {
    LaneList,
    Timeline,
}

/// Sub-mode for adding a new lane target
#[derive(Debug, Clone)]
enum TargetPickerState {
    Inactive,
    Active { options: Vec<AutomationTarget>, cursor: usize },
}

pub struct AutomationPane {
    keymap: Keymap,
    focus: AutomationFocus,
    // Timeline cursor
    pub(crate) cursor_tick: u32,
    cursor_value: f32,
    // Timeline viewport
    view_start_tick: u32,
    zoom_level: u8,
    snap_to_grid: bool,
    // Target picker sub-mode
    target_picker: TargetPickerState,
    pub(crate) selection_anchor_tick: Option<u32>,
}

impl AutomationPane {
    pub fn new(keymap: Keymap) -> Self {
        Self {
            keymap,
            focus: AutomationFocus::LaneList,
            cursor_tick: 0,
            cursor_value: 0.5,
            view_start_tick: 0,
            zoom_level: 3,
            snap_to_grid: true,
            target_picker: TargetPickerState::Inactive,
            selection_anchor_tick: None,
        }
    }

    fn ticks_per_cell(&self) -> u32 {
        crate::state::grid::ticks_per_cell(self.zoom_level)
    }

    fn snap_tick(&self, tick: u32) -> u32 {
        if self.snap_to_grid {
            crate::state::grid::snap_to_grid(tick, self.zoom_level)
        } else {
            tick
        }
    }

    /// Get the currently selected lane id
    pub(crate) fn selected_lane_id(&self, state: &AppState) -> Option<AutomationLaneId> {
        state.session.automation.selected().map(|l| l.id)
    }

    /// Returns the selection region as (lane_id, start_tick, end_tick), or None if no selection.
    pub(crate) fn selection_region(&self, state: &AppState) -> Option<(AutomationLaneId, u32, u32)> {
        let lane_id = self.selected_lane_id(state)?;
        let anchor_tick = self.selection_anchor_tick?;
        let (t0, t1) = crate::state::grid::normalize_tick_range(anchor_tick, self.cursor_tick);
        if t0 < t1 {
            Some((lane_id, t0, t1))
        } else {
            None
        }
    }
}

impl Pane for AutomationPane {
    fn id(&self) -> &'static str {
        "automation"
    }

    fn handle_action(&mut self, action: ActionId, event: &InputEvent, state: &AppState) -> Action {
        self.handle_action_impl(action, event, state)
    }

    fn render(&mut self, area: Rect, buf: &mut RenderBuf, state: &AppState) {
        let rect = center_rect(area, 100.min(area.width), 30.min(area.height));

        // Title
        let inst_name = state.instruments.selected_instrument()
            .map(|i| format!("Inst {} ({})", i.id, &i.name))
            .unwrap_or_else(|| "—".to_string());
        let title = format!(" Automation: {} ", inst_name);

        let border_color = Color::CYAN;
        let border_style = Style::new().fg(border_color);
        let title_style = Style::new().fg(border_color);
        let inner = buf.draw_block(rect, &title, border_style, title_style);

        if inner.height < 5 {
            return;
        }

        // Split inner area: top half for lane list, bottom half for timeline
        let lane_list_height = (inner.height / 3).max(3);
        let timeline_height = inner.height.saturating_sub(lane_list_height + 1);

        let lane_list_area = Rect::new(inner.x, inner.y, inner.width, lane_list_height);
        let separator_y = inner.y + lane_list_height;
        let timeline_area = Rect::new(inner.x, separator_y + 1, inner.width, timeline_height);

        // Render lane list
        self.render_lane_list(buf, lane_list_area, state);

        // Separator
        let sep_style = Style::new().fg(Color::DARK_GRAY);
        let timeline_title = state.session.automation.selected()
            .map(|l| {
                let (min, max) = (l.min_value, l.max_value);
                format!("─ {} ({:.1}–{:.1}) ", l.target.name(), min, max)
            })
            .unwrap_or_else(|| "─".to_string());

        for x in inner.x..inner.x + inner.width {
            buf.set_cell(x, separator_y, '─', sep_style);
        }
        // Overlay title on separator
        let title_overlay_style = Style::new().fg(Color::CYAN);
        for (i, ch) in timeline_title.chars().enumerate() {
            let x = inner.x + 1 + i as u16;
            if x >= inner.x + inner.width { break; }
            buf.set_cell(x, separator_y, ch, title_overlay_style);
        }

        // Render timeline
        self.render_timeline(buf, timeline_area, state);

        // Render target picker overlay (if active)
        self.render_target_picker(buf, rect, state);
    }

    fn keymap(&self) -> &Keymap {
        &self.keymap
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::AppState;
    use crate::ui::{InputEvent, KeyCode, Modifiers};

    fn dummy_event() -> InputEvent {
        InputEvent::new(KeyCode::Char('x'), Modifiers::default())
    }

    #[test]
    fn automation_pane_id() {
        let pane = AutomationPane::new(Keymap::new());
        assert_eq!(pane.id(), "automation");
    }

    #[test]
    fn switch_focus_toggles() {
        use crate::ui::action_id::{ActionId, AutomationActionId};
        let mut pane = AutomationPane::new(Keymap::new());
        let state = AppState::new();
        assert_eq!(pane.focus, AutomationFocus::LaneList);

        pane.handle_action(ActionId::Automation(AutomationActionId::SwitchFocus), &dummy_event(), &state);
        assert_eq!(pane.focus, AutomationFocus::Timeline);

        pane.handle_action(ActionId::Automation(AutomationActionId::SwitchFocus), &dummy_event(), &state);
        assert_eq!(pane.focus, AutomationFocus::LaneList);
    }

    #[test]
    fn timeline_cursor_moves() {
        use crate::ui::action_id::{ActionId, AutomationActionId};
        let mut pane = AutomationPane::new(Keymap::new());
        let state = AppState::new();
        pane.focus = AutomationFocus::Timeline;

        let start_tick = pane.cursor_tick;
        pane.handle_action(ActionId::Automation(AutomationActionId::Right), &dummy_event(), &state);
        assert!(pane.cursor_tick > start_tick);

        pane.handle_action(ActionId::Automation(AutomationActionId::Left), &dummy_event(), &state);
        assert_eq!(pane.cursor_tick, start_tick);
    }

    #[test]
    fn add_lane_opens_target_picker() {
        use crate::ui::action_id::{ActionId, AutomationActionId};
        let mut pane = AutomationPane::new(Keymap::new());
        let state = AppState::new();
        pane.handle_action(ActionId::Automation(AutomationActionId::AddLane), &dummy_event(), &state);
        assert!(matches!(pane.target_picker, TargetPickerState::Active { .. }));
    }
}
