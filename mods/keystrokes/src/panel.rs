//! The settings menu, in the Waypoints look (`tools/gen_menu.py`). Cinnabar draws it; the mod
//! sends the values and reacts to events.

use crate::config::{Corner, MAX_SCALE, MIN_SCALE, PALETTE, Settings, valid_key};
use crate::menu::{self, Value};

/// What a handled event asks the frame loop to do next.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Effect {
    pub save: bool,
    pub republish: bool,
    pub rebind: bool,
    /// Open the drag-and-resize position editor.
    pub edit: bool,
}


impl Effect {
    pub fn merge(self, other: Effect) -> Effect {
        Effect {
            save: self.save || other.save,
            republish: self.republish || other.republish,
            rebind: self.rebind || other.rebind,
            edit: self.edit || other.edit,
        }
    }
}

/// How long a pressed preset button says "Done!".
const DONE_SECONDS: f32 = 2.0;

#[derive(Default)]
pub struct PanelState {
    capturing: bool,
    /// The open tab of the menu.
    tab: usize,
    /// "Move with the mouse": the menu hides and the overlay follows a drag.
    pub moving: bool,
    /// The button that was just pressed, and how long it keeps saying so.
    done: Option<(&'static str, f32)>,
    notice: Option<&'static str>,
}


fn color_slot<'a>(settings: &'a mut Settings, id: &str) -> Option<&'a mut usize> {
    match id {
        "key_color" => Some(&mut settings.key_color),
        "text_color" => Some(&mut settings.text_color),
        "pressed_color" => Some(&mut settings.pressed_color),
        "pressed_text_color" => Some(&mut settings.pressed_text_color),
        _ => None,
    }
}

impl PanelState {
    pub fn is_capturing(&self) -> bool {
        self.capturing
    }

    pub fn take_notice(&mut self) -> Option<&'static str> {
        self.notice.take()
    }

    /// Counts down the "Done!" label; republishes once when it goes back to normal.
    pub fn tick(&mut self, seconds: f32) -> Effect {
        let Some((id, left)) = self.done else {
            return Effect::default();
        };
        let left = left - if seconds.is_finite() { seconds.max(0.0) } else { 0.0 };
        if left > 0.0 {
            self.done = Some((id, left));
            Effect::default()
        } else {
            self.done = None;
            Effect { republish: true, ..Effect::default() }
        }
    }

    fn label(&self, id: &str, base: &'static str) -> &'static str {
        if self.done.is_some_and(|(done, _)| done == id) { "Done!" } else { base }
    }

    fn mark_done(&mut self, id: &'static str, notice: &'static str) {
        self.done = Some((id, DONE_SECONDS));
        self.notice = Some(notice);
    }

    /// Handles one panel event.
    pub fn event(&mut self, settings: &mut Settings, id: &str, value: f32) -> Effect {
        let before = settings.clone();
        let on = value >= 0.5;
        let index = if value.is_finite() { value.max(0.0) as usize } else { 0 };
        if let Some(tab) = menu::tab_event(id) {
            self.tab = tab;
            return Effect { republish: true, ..Effect::default() };
        }
        match id {
            "move" | "move_done" => {
                self.moving = id == "move";
                return Effect { republish: true, ..Effect::default() };
            }
            "menu_key" => {
                self.capturing = !self.capturing;
                return Effect { republish: true, ..Effect::default() };
            }
            "pos_x" | "pos_y" => {
                let current = settings.position.unwrap_or(settings.corner.fractions());
                let v = (value / 100.0).clamp(0.0, 1.0);
                settings.position = Some(if id == "pos_x" { [v, current[1]] } else { [current[0], v] });
            }
            "top_left" | "top_right" | "bottom_left" | "bottom_right" => {
                let (corner, done, notice) = match id {
                    "top_left" => (Corner::TopLeft, "top_left", "Keystrokes: moved to the top left. Done!"),
                    "top_right" => (Corner::TopRight, "top_right", "Keystrokes: moved to the top right. Done!"),
                    "bottom_left" => {
                        (Corner::BottomLeft, "bottom_left", "Keystrokes: moved to the bottom left. Done!")
                    }
                    _ => (Corner::BottomRight, "bottom_right", "Keystrokes: moved to the bottom right. Done!"),
                };
                settings.snap_to(corner);
                self.mark_done(done, notice);
            }
            "reset_colors" => {
                settings.reset_colors();
                self.mark_done("reset_colors", "Keystrokes: colours reset. Done!");
            }
            "scale" => settings.scale = f64::from(value),
            "opacity" => settings.opacity = f64::from(value),
            "show_mouse" => settings.show_mouse = on,
            "show_cps" => settings.show_cps = on,
            "show_space" => settings.show_space = on,
            _ => match color_slot(settings, id) {
                Some(slot) => *slot = index.min(PALETTE.len() - 1),
                None => return Effect::default(),
            },
        }
        settings.sanitize();
        Effect {
            save: *settings != before,
            // Always republish so the host's retained panel shows the new value.
            republish: true,
            rebind: false,
            edit: false,
        }
    }

    /// While the menu key is waiting, the next key pressed becomes it; Escape cancels.
    pub fn capture(&mut self, settings: &mut Settings, pressed: &[String]) -> Effect {
        if !self.capturing {
            return Effect::default();
        }
        for key in pressed {
            if key == "Escape" {
                // Escape clears the key: F10 opens the menu again.
                self.capturing = false;
                settings.menu_key.clear();
                return Effect { save: true, republish: true, rebind: true, edit: false };
            }
            // Left click operates the panel, including the keycap that started the capture.
            if key.starts_with("Mouse") {
                continue;
            }
            if valid_key(key) {
                settings.menu_key = key.clone();
                self.capturing = false;
                return Effect { save: true, republish: true, rebind: true, edit: false };
            }
        }
        Effect::default()
    }
}

const PALETTE_NAMES: [&str; 8] = {
    let mut names = [""; 8];
    let mut i = 0;
    while i < 8 {
        names[i] = PALETTE[i].0;
        i += 1;
    }
    names
};

/// The whole menu for the current settings (the bundle, or a changed menu key).
pub fn build(settings: &Settings, state: &PanelState) -> String {
    let value = |id: &str| value(settings, state, id);
    menu::build(&settings.menu_key, state.capturing.then_some("menu_key"), state.tab, state.moving, &value)
}

/// Only the values, for a menu the host already has.
pub fn patch(settings: &Settings, state: &PanelState) -> String {
    let value = |id: &str| value(settings, state, id);
    menu::patch(state.capturing.then_some("menu_key"), state.tab, state.moving, &value)
}

/// One control's value.
fn value<'a>(s: &'a Settings, state: &'a PanelState, id: &str) -> Value<'a> {
    let place = s.position.unwrap_or(s.corner.fractions());
    match id {
        "top_left" => Value::Button(Some(state.label("top_left", "Top Left"))),
        "top_right" => Value::Button(Some(state.label("top_right", "Top Right"))),
        "bottom_left" => Value::Button(Some(state.label("bottom_left", "Bottom Left"))),
        "bottom_right" => Value::Button(Some(state.label("bottom_right", "Bottom Right"))),
        "reset_colors" => Value::Button(Some(state.label("reset_colors", "Reset colors"))),
        "scale" => Value::Slider(s.scale, MIN_SCALE, MAX_SCALE, 0.25),
        "pos_x" => Value::Slider(f64::from(place[0]) * 100.0, 0.0, 100.0, 1.0),
        "pos_y" => Value::Slider(f64::from(place[1]) * 100.0, 0.0, 100.0, 1.0),
        "opacity" => Value::Slider(s.opacity, 0.0, 100.0, 5.0),
        "key_color" => Value::Choice(s.key_color, &PALETTE_NAMES),
        "text_color" => Value::Choice(s.text_color, &PALETTE_NAMES),
        "pressed_color" => Value::Choice(s.pressed_color, &PALETTE_NAMES),
        "pressed_text_color" => Value::Choice(s.pressed_text_color, &PALETTE_NAMES),
        "show_mouse" => Value::Toggle(s.show_mouse),
        "show_cps" => Value::Toggle(s.show_cps),
        "show_space" => Value::Toggle(s.show_space),
        "menu_key" => Value::Key(&s.menu_key),
        _ => Value::Button(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    #[test]
    fn the_menu_lists_every_option() {
        let json: Value = serde_json::from_str(&build(&Settings::default(), &PanelState::default())).unwrap();
        assert_eq!(json["toggle_key"], "F24");
        let controls = json["controls"].as_array().unwrap();
        let ids: Vec<&str> = controls.iter().map(|c| c["id"].as_str().unwrap()).collect();
        for id in ["tab_0", "top_right", "bottom_left", "scale", "pos_x", "key_color", "opacity", "show_cps", "menu_key"] {
            assert!(ids.contains(&id), "{id} missing");
        }
        let key_color = controls.iter().find(|c| c["id"] == "key_color").unwrap();
        assert_eq!(key_color["index"], 2);
        assert_eq!(key_color["options"].as_array().unwrap().len(), 8);
        if std::env::var_os("KEYSTROKES_DUMP").is_some() {
            std::fs::write("/home/claude/keystrokes-panel.json", build(&Settings::default(), &PanelState::default()))
                .unwrap();
        }
    }

    #[test]
    fn presets_move_the_overlay_and_say_done() {
        let mut settings = Settings::default();
        settings.position = Some([0.5, 0.5]);
        let mut state = PanelState::default();
        let effect = state.event(&mut settings, "bottom_right", 1.0);
        assert!(effect.save && effect.republish);
        assert_eq!(settings.corner, Corner::BottomRight);
        assert_eq!(settings.position, None);
        assert!(build(&settings, &state).contains("\"Done!\""));
        assert!(state.take_notice().is_some());
        assert!(state.tick(1.0) == Effect::default());
        assert!(state.tick(1.5).republish);
        assert!(!build(&settings, &state).contains("Done!"));
    }

    #[test]
    fn colours_sliders_and_toggles_update_the_settings() {
        let mut settings = Settings::default();
        let mut state = PanelState::default();
        state.event(&mut settings, "key_color", 5.0);
        state.event(&mut settings, "pressed_text_color", 42.0);
        state.event(&mut settings, "opacity", 75.0);
        state.event(&mut settings, "scale", 9.0);
        state.event(&mut settings, "show_cps", 0.0);
        assert_eq!(settings.key_color, 5);
        assert_eq!(settings.pressed_text_color, 7);
        assert_eq!(settings.opacity, 75.0);
        assert_eq!(settings.scale, MAX_SCALE);
        assert!(!settings.show_cps);
        state.event(&mut settings, "reset_colors", 1.0);
        assert_eq!(settings.key_color, Settings::default().key_color);
        state.event(&mut settings, "pos_x", 25.0);
        assert_eq!(settings.position, Some([0.25, 0.0]));
        state.event(&mut settings, "tab_1", 1.0);
        assert_eq!(state.tab, 1);
        assert_eq!(state.event(&mut settings, "nope", 1.0), Effect::default());
    }

    #[test]
    fn the_menu_key_can_be_rebound() {
        let mut settings = Settings::default();
        let mut state = PanelState::default();
        state.event(&mut settings, "menu_key", 1.0);
        assert!(state.is_capturing());
        assert_eq!(state.capture(&mut settings, &["MouseLeft".into()]), Effect::default());
        let effect = state.capture(&mut settings, &["KeyK".into()]);
        assert!(effect.save && effect.rebind);
        assert_eq!(settings.menu_key, "KeyK");
        state.event(&mut settings, "menu_key", 1.0);
        state.capture(&mut settings, &["Escape".into()]);
        assert!(!state.is_capturing());
        assert_eq!(settings.menu_key, "", "Escape clears it: F10 opens the menu");
    }
}
