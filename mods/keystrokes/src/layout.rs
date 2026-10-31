//! What the keystrokes overlay draws: one panel of key cells on the mod's HUD layer, laid out
//! like a keyboard, placed and coloured from the settings. The document changes only with the
//! settings; held keys and CPS are bindings.

use crate::config::{Settings, rgba};
use crate::json::{push_u, rgba as write_rgba};
use crate::menu::num;

/// One key on screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Key {
    /// The host's physical key name.
    pub name: &'static str,
    /// What the cell says.
    pub label: &'static str,
    /// Cell rectangle in unscaled GUI pixels: x, y, width, height.
    pub rect: [u32; 4],
    /// Whether the cell shows a clicks-per-second line under its label.
    pub cps: bool,
}

const SIDE: u32 = 22;
const GAP: u32 = 2;
const ROW: u32 = SIDE + GAP;
const WIDE: u32 = (CARD_WIDTH - GAP) / 2;
const SPACE_HEIGHT: u32 = 10;

/// The card's width: three keys and their gaps.
pub const CARD_WIDTH: u32 = SIDE * 3 + GAP * 2;
/// Every physical key the overlay can show, so the frame loop can ask for exactly these.
pub const KEY_NAMES: [&str; 7] = ["KeyW", "KeyA", "KeyS", "KeyD", "MouseLeft", "MouseRight", "Space"];

/// The keys the settings ask for, top to bottom, with the rows below a hidden one moved up.
pub fn keys(settings: &Settings) -> Vec<Key> {
    let mut keys = vec![
        Key { name: "KeyW", label: "W", rect: [ROW, 0, SIDE, SIDE], cps: false },
        Key { name: "KeyA", label: "A", rect: [0, ROW, SIDE, SIDE], cps: false },
        Key { name: "KeyS", label: "S", rect: [ROW, ROW, SIDE, SIDE], cps: false },
        Key { name: "KeyD", label: "D", rect: [ROW * 2, ROW, SIDE, SIDE], cps: false },
    ];
    let mut y = ROW * 2;
    if settings.show_mouse {
        keys.push(Key { name: "MouseLeft", label: "LMB", rect: [0, y, WIDE, SIDE], cps: settings.show_cps });
        keys.push(Key {
            name: "MouseRight",
            label: "RMB",
            rect: [WIDE + GAP, y, WIDE, SIDE],
            cps: settings.show_cps,
        });
        y += ROW;
    }
    if settings.show_space {
        keys.push(Key { name: "Space", label: "", rect: [0, y, CARD_WIDTH, SPACE_HEIGHT], cps: false });
    }
    keys
}

/// Everything the overlay shows in one frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct View {
    /// Whether each of `KEY_NAMES` is held, in the same order.
    pub held: [bool; 7],
    pub left_cps: u32,
    pub right_cps: u32,
}

impl View {
    fn holds(&self, name: &str) -> bool {
        KEY_NAMES
            .iter()
            .position(|known| *known == name)
            .is_some_and(|index| self.held[index])
    }
}

/// Opaque enough to read a held key at any idle opacity.
const PRESSED_ALPHA: f64 = 0.9;
/// Gap between the overlay and the screen edge when pinned to a corner, in GUI pixels.
const MARGIN: f32 = 6.0;
pub const SCREEN: &str = "keystrokes.hud";

/// The overlay's size in GUI pixels at the settings' scale.
pub fn size(settings: &Settings) -> [f32; 2] {
    let bottom = keys(settings).iter().map(|k| k.rect[1] + k.rect[3]).max().unwrap_or(0);
    let k = settings.scale as f32;
    [CARD_WIDTH as f32 * k, bottom as f32 * k]
}

/// The overlay's top-left corner on a screen of `viewport` GUI pixels.
pub fn top_left(settings: &Settings, viewport: [f32; 2]) -> [f32; 2] {
    if let Some(at) = settings.position {
        return placed_offset(settings, viewport, at);
    }
    let [w, h] = size(settings);
    let anchor = settings.corner.anchor();
    [
        if anchor.ends_with("right") { viewport[0] - w - MARGIN } else { MARGIN },
        if anchor.starts_with("bottom") { viewport[1] - h - MARGIN } else { MARGIN },
    ]
}

/// The placement that puts the overlay's top-left corner at `corner`, kept on screen.
pub fn place_at(settings: &Settings, viewport: [f32; 2], corner: [f32; 2]) -> [f32; 2] {
    let [w, h] = size(settings);
    let free = [(viewport[0] - w).max(1.0), (viewport[1] - h).max(1.0)];
    [(corner[0] / free[0]).clamp(0.0, 1.0), (corner[1] / free[1]).clamp(0.0, 1.0)]
}

/// Where the overlay's top-left corner goes for a placed (not pinned) overlay.
fn placed_offset(settings: &Settings, viewport: [f32; 2], at: [f32; 2]) -> [f32; 2] {
    let [w, h] = size(settings);
    [((viewport[0] - w).max(0.0) * at[0]).round(), ((viewport[1] - h).max(0.0) * at[1]).round()]
}

fn light(index: usize) -> bool {
    let [r, g, b, _] = rgba(index, 1.0);
    0.3 * r + 0.59 * g + 0.11 * b > 0.5
}

/// The HUD document, already escaped to sit inside a JSON string (escaping it afterwards
/// would cost more fuel than building it): one panel of key cells, each coloured by its own
/// bindings.
pub fn document(settings: &Settings, viewport: Option<[f32; 2]>) -> String {
    let k = settings.scale as f32;
    let [w, h] = size(settings);
    let mut o = String::with_capacity(4096);
    o.push_str("{\\\"namespace\\\":\\\"keystrokes\\\",\\\"hud\\\":{\\\"type\\\":\\\"screen\\\",\\\"size\\\":[\\\"100%\\\",\\\"100%\\\"],\\\"controls\\\":[{\\\"ks\\\":{\\\"type\\\":\\\"panel\\\",\\\"size\\\":[");
    num(&mut o, f64::from(w));
    o.push(',');
    num(&mut o, f64::from(h));
    o.push_str("],");
    match (settings.position, viewport) {
        (Some(at), Some(view)) => {
            let [x, y] = placed_offset(settings, view, at);
            o.push_str("\\\"anchor_from\\\":\\\"top_left\\\",\\\"anchor_to\\\":\\\"top_left\\\",\\\"offset\\\":[");
            num(&mut o, f64::from(x));
            o.push(',');
            num(&mut o, f64::from(y));
            o.push(']');
        }
        _ => {
            let corner = if settings.position.is_some() { crate::config::Corner::TopLeft } else { settings.corner };
            let anchor = corner.anchor();
            o.push_str("\\\"anchor_from\\\":");
            qe(&mut o, anchor);
            o.push_str(",\\\"anchor_to\\\":");
            qe(&mut o, anchor);
            o.push_str(",\\\"offset\\\":[");
            num(&mut o, f64::from(if anchor.ends_with("right") { -MARGIN } else { MARGIN }));
            o.push(',');
            num(&mut o, f64::from(if anchor.starts_with("bottom") { -MARGIN } else { MARGIN }));
            o.push(']');
        }
    }
    let shadow = if light(settings.text_color) && light(settings.pressed_text_color) { "true" } else { "false" };
    o.push_str(",\\\"controls\\\":[");
    for (index, key) in keys(settings).iter().enumerate() {
        if index > 0 {
            o.push(',');
        }
        let [x, y, cw, ch] = key.rect.map(|v| v as f32 * k);
        let i = slot(key.name);
        o.push_str("{\\\"k");
        push_u(&mut o, i as u64);
        o.push_str("\\\":{\\\"type\\\":\\\"panel\\\",\\\"size\\\":[");
        num(&mut o, f64::from(cw));
        o.push(',');
        num(&mut o, f64::from(ch));
        o.push_str("],\\\"offset\\\":[");
        num(&mut o, f64::from(x));
        o.push(',');
        num(&mut o, f64::from(y));
        o.push_str("],\\\"anchor_from\\\":\\\"top_left\\\",\\\"anchor_to\\\":\\\"top_left\\\",\\\"controls\\\":[");
        // Background.
        o.push_str("{\\\"bg\\\":{\\\"type\\\":\\\"custom\\\",\\\"renderer\\\":\\\"cinnabar_rounded_rectangle\\\",\\\"radius\\\":");
        num(&mut o, f64::from(1.5 * k));
        o.push_str(",\\\"color\\\":[0,0,0,0.5],\\\"size\\\":[\\\"100%\\\",\\\"100%\\\"],\\\"bindings\\\":[{\\\"binding_name\\\":\\\"#k");
        push_u(&mut o, i as u64);
        o.push_str("_bg\\\",\\\"binding_name_override\\\":\\\"#color\\\"}]}}");
        // Label, and the CPS line under it.
        let text_h = 9.0 * k;
        let (label_y, cps) = if key.cps { (ch * 0.5 - text_h * 0.95, true) } else { ((ch - text_h) * 0.5 + 0.5 * k, false) };
        if !key.label.is_empty() {
            label(&mut o, "l", key.label, None, i, [cw, text_h], [0.0, label_y], k, shadow);
        }
        if cps {
            label(&mut o, "c", "", Some("_cps"), i, [cw, 7.0 * k], [0.0, ch * 0.5 + 0.5 * k], k * 0.6, shadow);
        }
        o.push_str("]}}");
    }
    o.push_str("]}}]}}");
    o
}

fn label(o: &mut String, name: &str, text: &str, bound: Option<&str>, i: usize, size: [f32; 2], offset: [f32; 2], scale: f32, shadow: &str) {
    o.push_str(",{\\\"");
    o.push_str(name);
    o.push_str("\\\":{\\\"type\\\":\\\"label\\\",\\\"text\\\":");
    match bound {
        Some(suffix) => {
            o.push_str("\\\"#k");
            push_u(o, i as u64);
            o.push_str(suffix);
            o.push_str("\\\"");
        }
        None => qe(o, text),
    }
    o.push_str(",\\\"localize\\\":false,\\\"text_alignment\\\":\\\"center\\\",\\\"shadow\\\":");
    o.push_str(shadow);
    o.push_str(",\\\"font_scale_factor\\\":");
    num(o, f64::from(scale));
    o.push_str(",\\\"size\\\":[");
    num(o, f64::from(size[0]));
    o.push(',');
    num(o, f64::from(size[1]));
    o.push_str("],\\\"offset\\\":[");
    num(o, f64::from(offset[0]));
    o.push(',');
    num(o, f64::from(offset[1]));
    o.push_str("],\\\"anchor_from\\\":\\\"top_left\\\",\\\"anchor_to\\\":\\\"top_left\\\",\\\"bindings\\\":[");
    if let Some(suffix) = bound {
        o.push_str("{\\\"binding_name\\\":\\\"#k");
        push_u(o, i as u64);
        o.push_str(suffix);
        o.push_str("\\\"},");
    }
    o.push_str("{\\\"binding_name\\\":\\\"#k");
    push_u(o, i as u64);
    o.push_str("_fg\\\",\\\"binding_name_override\\\":\\\"#color\\\"}]}}");
}

/// A string inside the escaped document.
fn qe(o: &mut String, text: &str) {
    o.push_str("\\\"");
    o.push_str(text);
    o.push_str("\\\"");
}

/// The index of a key in `KEY_NAMES`, which names its bindings.
fn slot(name: &str) -> usize {
    KEY_NAMES.iter().position(|known| *known == name).unwrap_or(0)
}

/// Every key's colours and CPS text for one view, as JSON object members.
fn values(o: &mut String, view: &View, settings: &Settings) {
    let idle = settings.opacity / 100.0;
    let mut first = true;
    for key in keys(settings) {
        let i = slot(key.name);
        let held = view.holds(key.name);
        for (suffix, color) in [
            ("_bg", if held { rgba(settings.pressed_color, PRESSED_ALPHA) } else { rgba(settings.key_color, idle) }),
            ("_fg", rgba(if held { settings.pressed_text_color } else { settings.text_color }, 1.0)),
        ] {
            if !first {
                o.push(',');
            }
            first = false;
            o.push_str("\"#k");
            push_u(o, i as u64);
            o.push_str(suffix);
            o.push_str("\":");
            write_rgba(o, color);
        }
        if key.cps {
            o.push_str(",\"#k");
            push_u(o, i as u64);
            o.push_str("_cps\":\"");
            push_u(o, u64::from(if key.name == "MouseLeft" { view.left_cps } else { view.right_cps }));
            o.push_str(" CPS\"");
        }
    }
}

/// The whole layer for `hud.set-surface`.
pub fn surface(view: &View, settings: &Settings, viewport: Option<[f32; 2]>) -> String {
    let doc = document(settings, viewport);
    let mut o = String::with_capacity(doc.len() + 1024);
    o.push_str("{\"screen\":\"");
    o.push_str(SCREEN);
    o.push_str("\",\"document\":\"");
    o.push_str(&doc);
    o.push_str("\",\"bindings\":{");
    values(&mut o, view, settings);
    o.push_str("}}");
    o
}

/// A patch with only the changed view's values.
pub fn patch(view: &View, settings: &Settings) -> String {
    let mut o = String::with_capacity(1024);
    o.push_str("{\"binding_changes\":{");
    values(&mut o, view, settings);
    o.push_str("}}");
    o
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Corner;
    use serde_json::Value;

    fn parse(text: &str) -> Value {
        serde_json::from_str(text).unwrap()
    }

    #[test]
    fn the_full_layout_fits_the_card() {
        let settings = Settings::default();
        let keys = keys(&settings);
        assert_eq!(keys.len(), 7);
        for key in &keys {
            assert!(key.rect[0] + key.rect[2] <= CARD_WIDTH, "{} spills out", key.label);
        }
        assert_eq!(keys[0].rect[0], keys[2].rect[0], "W sits above S");
    }

    #[test]
    fn hiding_the_mouse_row_moves_the_space_bar_up() {
        let mut settings = Settings::default();
        settings.show_mouse = false;
        let keys = keys(&settings);
        assert_eq!(keys.len(), 5);
        assert_eq!(keys[4].name, "Space");
        assert_eq!(keys[4].rect[1], ROW * 2);
        settings.show_space = false;
        assert_eq!(super::keys(&settings).len(), 4);
    }

    #[test]
    fn the_layer_is_valid_and_coloured_from_the_settings() {
        let mut settings = Settings::default();
        settings.snap_to(Corner::BottomRight);
        settings.scale = 1.5;
        settings.key_color = 3;
        settings.opacity = 80.0;
        let mut view = View::default();
        view.held[0] = true;
        view.left_cps = 7;
        let json = parse(&surface(&view, &settings, Some([427.0, 240.0])));
        assert_eq!(json["screen"], SCREEN);
        assert_eq!(json["bindings"]["#k0_bg"], serde_json::json!([1, 1, 1, 0.9]), "held W uses the pressed colour");
        assert_eq!(json["bindings"]["#k1_bg"], serde_json::json!([1, 0.33, 0.33, 0.8]), "idle A is red at 80%");
        assert_eq!(json["bindings"]["#k4_cps"], "7 CPS");
        let doc = parse(json["document"].as_str().unwrap());
        let ks = &doc["hud"]["controls"][0]["ks"];
        assert_eq!(ks["anchor_from"], "bottom_right");
        assert_eq!(ks["offset"], serde_json::json!([-6, -6]));
        assert_eq!(ks["size"][0], 105);
        assert_eq!(ks["controls"].as_array().unwrap().len(), 7);
        let patch = parse(&patch(&view, &settings));
        assert_eq!(patch["binding_changes"]["#k5_cps"], "0 CPS");
        if std::env::var_os("KEYSTROKES_DUMP").is_some() {
            std::fs::write("/home/claude/keystrokes-hud.json", surface(&view, &settings, Some([427.0, 240.0]))).unwrap();
        }
    }

    #[test]
    fn a_placed_overlay_uses_the_free_space_and_cps_can_be_hidden() {
        let mut settings = Settings::default();
        settings.position = Some([0.5, 1.0]);
        settings.show_cps = false;
        let json = parse(&surface(&View::default(), &settings, Some([427.0, 240.0])));
        let doc = parse(json["document"].as_str().unwrap());
        let ks = &doc["hud"]["controls"][0]["ks"];
        assert_eq!(ks["anchor_from"], "top_left");
        let [w, h] = size(&settings);
        assert_eq!(ks["offset"], serde_json::json!([((427.0 - w) * 0.5f32).round() as i64, (240.0 - h).round() as i64]));
        assert!(json["bindings"].get("#k4_cps").is_none());
    }

    #[test]
    fn dragging_places_the_overlay_where_the_corner_goes() {
        let mut settings = Settings::default();
        let view = [427.0, 240.0];
        assert_eq!(top_left(&settings, view), [6.0, 6.0]);
        settings.snap_to(Corner::BottomRight);
        let [w, h] = size(&settings);
        assert_eq!(top_left(&settings, view), [427.0 - w - 6.0, 240.0 - h - 6.0]);
        settings.position = Some(place_at(&settings, view, [100.0, 50.0]));
        let at = top_left(&settings, view);
        assert!((at[0] - 100.0).abs() <= 0.5 && (at[1] - 50.0).abs() <= 0.5, "{at:?}");
        assert_eq!(place_at(&settings, view, [-50.0, 999.0]), [0.0, 1.0], "kept on screen");
    }
}
