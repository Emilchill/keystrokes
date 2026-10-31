#!/usr/bin/env python3
"""Generates the Keystrokes menu (src/menu.json, src/menu.escaped, src/menu_controls.rs)."""
import pathlib, sys
sys.path.insert(0, str(pathlib.Path(__file__).parent))
from menukit import Menu

m = Menu("keystrokes", "Keystrokes", subtitle="+ CPS", height=262,
         move_hint="Drag the keys with the mouse.")
m.tab("Layout",
      [("head", "Position"), ("button", "move", "Move with the mouse", "accent"),
       ("button", "top_left", "Top Left"), ("button", "top_right", "Top Right"),
       ("button", "bottom_left", "Bottom Left"), ("button", "bottom_right", "Bottom Right"),
       ("note", "Or set X and Y.")],
      [("head", "Size and place"),
       ("slider", "scale", "Size"), ("slider", "pos_x", "X (%)"), ("slider", "pos_y", "Y (%)"),
       ("head", "Menu"), ("key", "menu_key", "Menu key")])
m.tab("Look",
      [("head", "Colors"),
       ("choice", "key_color", "Key"), ("choice", "text_color", "Text"),
       ("choice", "pressed_color", "Pressed key"), ("choice", "pressed_text_color", "Pressed text"),
       ("button", "reset_colors", "Reset colors")],
      [("head", "Show"),
       ("slider", "opacity", "Background (%)"),
       ("toggle", "show_mouse", "Mouse buttons"), ("toggle", "show_cps", "Clicks per second"),
       ("toggle", "show_space", "Space bar")])
m.generate(pathlib.Path(__file__).resolve().parents[1] / "mods/keystrokes/src")
