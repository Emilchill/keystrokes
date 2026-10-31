# Keystrokes for Cinnabar

Shows your movement keys, jump and mouse buttons in the top-left corner, lighting up while
you hold them, with clicks per second under each mouse button.

```
     [ W ]
[ A ][ S ][ D ]
[ LMB  ][ RMB  ]
   7 CPS  0 CPS
[    space    ]
```

Held keys turn white with dark text. CPS counts the clicks of the last second.

The menu (open it from **Mods Core**: F10, then click Keystrokes; or set your own key, empty by default) has two tabs: **Layout** (**Move with the mouse**: the menu hides and you drag
the keys anywhere, then press Done; corner presets, size, and X/Y sliders) and **Look** (colours, background opacity, which parts show).

Needs a Cinnabar build with the new mod API (October 25, 2026 or later).

## Building

`python scripts/build-mod.py` writes `dist/keystrokes.wasm`. Import it in Cinnabar from
Settings → Mods → Import.

## Permissions

Controls, to see which keys are held (it only reads them; nothing is blocked or pressed),
and HUD, to draw the overlay.
