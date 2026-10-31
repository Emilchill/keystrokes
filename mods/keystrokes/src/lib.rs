//! Keystrokes for Cinnabar: WASD, jump and the mouse buttons on screen, with CPS, plus a
//! menu to move, resize and recolour the overlay.
//!
//! Everything except the frame loop is plain Rust, so it is unit-tested on the host.
#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]

mod config;
mod cps;
mod json;
mod layout;
mod menu;
mod mods_core;
mod menu_controls;
mod panel;

#[cfg(target_arch = "wasm32")]
mod guest;

/// The menu with default settings, embedded in the component as its panel bundle.
pub fn menu_panel() -> String {
    panel::build(&config::Settings::default(), &panel::PanelState::default())
}

/// The bundle route of the menu's surface.
pub const MENU_ROUTE: &str = menu::ROUTE;
