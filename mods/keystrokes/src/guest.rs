//! The frame loop: sample the keys, count clicks, run the menu, and redraw only on change.

use crate::config::Settings;
use crate::cps::Counter;
use crate::layout::{self, KEY_NAMES, View};
use crate::panel::{Effect, PanelState};
use mod_api::bindings::{
    Guest,
    cinnabar::extension::{hud, input, panel, settings},
};
use std::cell::RefCell;

struct Mod;

#[derive(Default)]
struct State {
    settings: Settings,
    panel: PanelState,
    clock: f64,
    left: Counter,
    right: Counter,
    /// What the HUD shows now, so it is only rewritten when it changes.
    shown: Option<View>,
    /// The layer's document must be sent again (settings or screen size changed).
    rebuild: bool,
    /// The screen size the placed overlay was laid out for.
    viewport: Option<[f32; 2]>,
    /// While dragging: where the pointer holds the overlay, from its top-left corner.
    grab: Option<[f32; 2]>,
}

thread_local! {
    /// This mod's place in Mods Core's list.
    static CORE: RefCell<crate::mods_core::Link> = RefCell::new(crate::mods_core::Link::new(
        "keystrokes",
        "Keystrokes",
        env!("CARGO_PKG_VERSION"),
        "WASD, jump and mouse buttons on screen, with clicks per second. Move it with the mouse.",
    ));

    static STATE: RefCell<State> = RefCell::new(State::default());
}

impl Guest for Mod {
    fn init() {
        STATE.with_borrow_mut(|state| {
            state.settings = match settings::load() {
                Ok(text) => Settings::from_json(&text),
                Err(_) => Settings::default(),
            };
            state.rebuild = true;
            state.apply(Effect { republish: true, ..Effect::default() });
        });
    }

    fn frame() {
        STATE.with_borrow_mut(State::frame);
    }
}

impl State {
    fn frame(&mut self) {
        // While rebinding the menu key every key is read; otherwise only the overlay's own.
        let capturing = self.panel.is_capturing();
        let Ok(controls) = input::read_selected_controls(&input::Selection {
            keys_pressed: if capturing {
                None
            } else {
                let mut keys = vec!["MouseLeft".to_owned(), "MouseRight".to_owned()];
                if !self.settings.menu_key.is_empty() {
                    keys.push(self.settings.menu_key.clone());
                }
                Some(keys)
            },
            keys_held: Some(KEY_NAMES.iter().map(|name| (*name).to_owned()).collect()),
            events: true,
            keys_admitted: Some(Vec::new()),
        }) else {
            return;
        };

        // A rebound menu key opens and closes the menu (the embedded menu knows the default).
        if !capturing
            && crate::menu::has_own_key(&self.settings.menu_key)
            && controls.focused
            && controls.keys_pressed.iter().any(|k| *k == self.settings.menu_key)
        {
            core_toggle(controls.panel_open);
        }
        let mut effect = Effect::default();
        let mut kofi_changed = crate::menu::kofi_tick(controls.seconds);
        // Mods Core: a click on this mod in its list sets and opens the menu.
        let whole = || crate::panel::build(&self.settings, &self.panel);
        if core_frame(controls.panel_open, whole) == crate::mods_core::Event::Opened {
            kofi_changed = true;
        }
        for event in &controls.events {
            if crate::menu::is_kofi(&event.id) {
                let _ = hud::local_message(crate::menu::KOFI_MESSAGE);
                crate::menu::show_kofi_note();
                kofi_changed = true;
                continue;
            }
            effect = effect.merge(self.panel.event(&mut self.settings, &event.id, event.value));
        }
        effect = effect.merge(self.panel.capture(&mut self.settings, &controls.keys_pressed));
        effect = effect.merge(self.panel.tick(controls.seconds));
        if kofi_changed {
            effect = effect.merge(Effect { republish: true, ..Effect::default() });
        }
        // Sending the menu and laying out the overlay again do not share a frame.
        let menu_sent = effect.republish;
        self.apply(effect);
        if let Some(message) = self.panel.take_notice() {
            let _ = hud::local_message(message);
        }

        if self.panel.moving {
            if !controls.panel_open {
                // Closing the menu ends the move.
                self.panel.moving = false;
                self.grab = None;
            } else {
                self.drag(controls.pointer, &controls.keys_pressed, &controls.keys_held);
            }
        }
        // A placed overlay follows the screen size.
        if self.settings.position.is_some() {
            let viewport = hud::read_viewport().map(|v| [v.width, v.height]);
            if viewport != self.viewport {
                self.viewport = viewport;
                self.rebuild = true;
            }
        }

        let seconds = f64::from(controls.seconds);
        self.clock += if seconds.is_finite() { seconds.clamp(0.0, 1.0) } else { 0.0 };
        let presses = |name: &str| {
            if capturing { 0 } else { controls.keys_pressed.iter().filter(|key| *key == name).count() }
        };
        let view = View {
            held: std::array::from_fn(|index| controls.keys_held.iter().any(|key| key == KEY_NAMES[index])),
            left_cps: self.left.tick(self.clock, presses("MouseLeft")),
            right_cps: self.right.tick(self.clock, presses("MouseRight")),
        };
        if !(menu_sent && self.rebuild) {
            self.publish(view);
        }
    }

    /// Drags the overlay with the left mouse button while the menu is in its move mode.
    fn drag(&mut self, pointer: Option<(f32, f32)>, pressed: &[String], held: &[String]) {
        let (Some((px, py)), Some(view)) = (pointer, hud::read_viewport()) else {
            return;
        };
        let view = [view.width, view.height];
        let corner = layout::top_left(&self.settings, view);
        let [w, h] = layout::size(&self.settings);
        if pressed.iter().any(|k| k == "MouseLeft") {
            let inside = px >= corner[0] && px <= corner[0] + w && py >= corner[1] && py <= corner[1] + h;
            self.grab = inside.then_some([px - corner[0], py - corner[1]]);
        }
        let Some(grab) = self.grab else { return };
        if !held.iter().any(|k| k == "MouseLeft") {
            // Let go: keep the new place.
            self.grab = None;
            self.settings.sanitize();
            let _ = settings::save(&self.settings.to_json());
            return;
        }
        let target = [(px - grab[0]).round(), (py - grab[1]).round()];
        if target != corner {
            self.settings.position = Some(layout::place_at(&self.settings, view, target));
            self.viewport = Some(view);
            self.rebuild = true;
        }
    }

    fn apply(&mut self, effect: Effect) {
        if effect.save {
            let _ = settings::save(&self.settings.to_json());
            // Colours, size or placement changed: the layer is laid out again.
            self.rebuild = true;
        }
        if effect.republish {
            // Values go as a patch: the embedded menu already has the document. The whole menu
            // is sent only if the host has none.
            if panel::update_content(&crate::panel::patch(&self.settings, &self.panel)).is_err() {
                // No panel while the menu is closed: it is set whole when it opens.
            }
        }
    }

    fn publish(&mut self, view: View) {
        if self.rebuild {
            if hud::set_surface(&layout::surface(&view, &self.settings, self.viewport)).is_ok() {
                self.rebuild = false;
                self.shown = Some(view);
            }
            return;
        }
        if self.shown == Some(view) {
            return;
        }
        if hud::update_surface(&layout::patch(&view, &self.settings)).is_ok() {
            self.shown = Some(view);
        } else {
            self.rebuild = true;
        }
    }
}

mod_api::bindings::export!(Mod with_types_in mod_api::bindings);

/// Announces this mod to Mods Core, and sets and opens the menu when asked (or drops it when it
/// closes, so only one menu exists at a time; see `mods_core.rs`).
fn core_frame(panel_open: bool, whole_panel: impl FnOnce() -> String) -> crate::mods_core::Event {
    CORE.with_borrow_mut(|link| link.frame(panel_open, whole_panel))
}

/// The player's own menu key: opens the menu through Mods Core's hand-over, or closes it.
fn core_toggle(panel_open: bool) {
    CORE.with_borrow_mut(|link| link.toggle(panel_open));
}
