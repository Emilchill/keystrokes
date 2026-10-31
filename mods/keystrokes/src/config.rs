//! What the player can change: where the overlay sits, how big it is, its colours, and
//! which parts it shows. Saved to the mod's settings file.

use serde::{Deserialize, Serialize};

/// The colours offered in the menu; a choice control holds at most eight options.
pub const PALETTE: [(&str, [f32; 3]); 8] = [
    ("White", [1.0, 1.0, 1.0]),
    ("Gray", [0.5, 0.5, 0.5]),
    ("Black", [0.0, 0.0, 0.0]),
    ("Red", [1.0, 0.33, 0.33]),
    ("Green", [0.33, 1.0, 0.33]),
    ("Blue", [0.33, 0.55, 1.0]),
    ("Yellow", [1.0, 0.9, 0.25]),
    ("Purple", [0.75, 0.4, 1.0]),
];
const WHITE: usize = 0;
const BLACK: usize = 2;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Corner {
    #[default]
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

impl Corner {
    /// Where the corner is, as fractions of the free space.
    pub fn fractions(self) -> [f32; 2] {
        match self {
            Self::TopLeft => [0.0, 0.0],
            Self::TopRight => [1.0, 0.0],
            Self::BottomLeft => [0.0, 1.0],
            Self::BottomRight => [1.0, 1.0],
        }
    }

    pub fn anchor(self) -> &'static str {
        match self {
            Self::TopLeft => "top_left",
            Self::TopRight => "top_right",
            Self::BottomLeft => "bottom_left",
            Self::BottomRight => "bottom_right",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// The corner the overlay is pinned to, unless it has been dragged somewhere else.
    pub corner: Corner,
    /// Where the editor left it, as fractions of the free space on each axis.
    pub position: Option<[f32; 2]>,
    pub scale: f64,
    /// Indices into `PALETTE`.
    pub key_color: usize,
    pub text_color: usize,
    pub pressed_color: usize,
    pub pressed_text_color: usize,
    /// Opacity of an idle key's background, in percent.
    pub opacity: f64,
    pub show_mouse: bool,
    pub show_cps: bool,
    pub show_space: bool,
    pub menu_key: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            corner: Corner::TopLeft,
            position: None,
            scale: 1.0,
            key_color: BLACK,
            text_color: WHITE,
            pressed_color: WHITE,
            pressed_text_color: BLACK,
            opacity: 50.0,
            show_mouse: true,
            show_cps: true,
            show_space: true,
            // Empty: F10 opens the menu (Mod Menu).
            menu_key: String::new(),
        }
    }
}

pub const MIN_SCALE: f64 = 0.5;
pub const MAX_SCALE: f64 = 3.0;

impl Settings {
    pub fn from_json(text: &str) -> Self {
        let mut settings = serde_json::from_str::<Settings>(text).unwrap_or_default();
        settings.sanitize();
        settings
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".into())
    }

    /// Keeps every value inside what the HUD and the menu accept.
    pub fn sanitize(&mut self) {
        let defaults = Settings::default();
        self.scale = if self.scale.is_finite() {
            self.scale.clamp(MIN_SCALE, MAX_SCALE)
        } else {
            defaults.scale
        };
        self.opacity = if self.opacity.is_finite() {
            self.opacity.clamp(0.0, 100.0)
        } else {
            defaults.opacity
        };
        for color in [
            &mut self.key_color,
            &mut self.text_color,
            &mut self.pressed_color,
            &mut self.pressed_text_color,
        ] {
            if *color >= PALETTE.len() {
                *color = WHITE;
            }
        }
        if let Some(position) = &mut self.position {
            if position.iter().all(|v| v.is_finite()) {
                *position = position.map(|v| v.clamp(0.0, 1.0));
            } else {
                self.position = None;
            }
        }
        if !valid_key(&self.menu_key) {
            self.menu_key = defaults.menu_key;
        }
    }

    /// Pins the overlay to a corner, forgetting any dragged position.
    pub fn snap_to(&mut self, corner: Corner) {
        self.corner = corner;
        self.position = None;
    }

    pub fn reset_colors(&mut self) {
        let defaults = Settings::default();
        self.key_color = defaults.key_color;
        self.text_color = defaults.text_color;
        self.pressed_color = defaults.pressed_color;
        self.pressed_text_color = defaults.pressed_text_color;
        self.opacity = defaults.opacity;
    }
}

/// The host accepts bounded alphanumeric physical key names.
pub fn valid_key(key: &str) -> bool {
    !key.is_empty() && key.len() <= 32 && key.bytes().all(|b| b.is_ascii_alphanumeric())
}

/// A palette colour as host RGBA, at `alpha`.
pub fn rgba(index: usize, alpha: f64) -> [f32; 4] {
    let [r, g, b] = PALETTE.get(index).map_or([1.0; 3], |(_, rgb)| *rgb);
    [r, g, b, alpha.clamp(0.0, 1.0) as f32]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_defaults_match_the_original_look() {
        let s = Settings::default();
        assert_eq!(s.corner, Corner::TopLeft);
        assert_eq!(rgba(s.key_color, 0.5), [0.0, 0.0, 0.0, 0.5]);
        assert_eq!(rgba(s.pressed_color, 1.0), [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(s.menu_key, "");
    }

    #[test]
    fn corners_forget_a_dragged_position() {
        let mut s = Settings::default();
        s.position = Some([0.4, 0.6]);
        s.snap_to(Corner::BottomRight);
        assert_eq!(s.position, None);
        assert_eq!(s.corner.anchor(), "bottom_right");
    }

    #[test]
    fn junk_settings_are_repaired() {
        let s = Settings::from_json(
            r#"{"scale": 40, "opacity": -5, "key_color": 99, "position": [2, -1], "menu_key": "F 8!"}"#,
        );
        assert_eq!(s.scale, MAX_SCALE);
        assert_eq!(s.opacity, 0.0);
        assert_eq!(s.key_color, 0);
        assert_eq!(s.position, Some([1.0, 0.0]));
        assert_eq!(s.menu_key, "");
        let mut changed = s.clone();
        changed.reset_colors();
        assert_eq!(changed.key_color, Settings::default().key_color);
        assert_eq!(Settings::from_json(&s.to_json()), s, "round trip");
    }
}
