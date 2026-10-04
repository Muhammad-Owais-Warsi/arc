# Keymaps

One default file per OS — `default-macos.json` uses `cmd-` chords,
`default-windows.json` / `default-linux.json` use `ctrl-`. This split is
mandatory, not stylistic: gpui matches modifiers exactly, and on Windows
Ctrl reports `control` (the Win key is `platform`), so a `cmd-` chord can
never fire there. Same rule applies to your own `keymap.json`.

## Schema

A top-level array of sections:

```jsonc
[
  {
    "context": "RequestPlayground",   // optional; empty/absent = global
    "use_key_equivalents": true,       // optional; macOS non-QWERTY layouts
    "unbind": {                        // optional; processed before bindings
      "cmd-n": ["gpui::Unbind", "tabs::NewRequestTab"]
    },
    "bindings": {
      "cmd-s": "request::SaveRequest",          // plain action
      "cmd-1": ["tabs::ActivateTab", 0],        // action with argument
      "cmd-w": null                             // disabled chord
    }
  }
]
```

- Chords are whitespace-separated strokes, modifiers joined with `-`
  (`"cmd-k cmd-s"`, `"ctrl-shift-tab"`).
- Contexts form a stack from the focused element up; deeper wins
  (`RequestPlayground` beats global). Available: `RequestPlayground`,
  `FilePanel`, `EnvPanel`, `EnvPlayground`, `ResponsePanel`.
- Later files win over earlier ones: defaults, then your `keymap.json`.
- A broken section never breaks the rest — it is skipped with an error
  on stderr.

## Default chords

| Chord | Action |
|---|---|
| `cmd-n` | New request tab (`tabs::NewRequestTab`) |
| `cmd-w` | Close active tab (`tabs::CloseTab`) |
| `cmd-shift-t` | Reopen closed request (`tabs::ReopenClosedTab`) |
| `ctrl-tab` / `ctrl-shift-tab` | Next / previous tab |
| `cmd-1`…`cmd-9` | Activate tab by strip position |
| `cmd-,` | Open settings (`arc::OpenSettings`) |
| `cmd-q` | Quit (`arc::QuitArc`) |
| `cmd-shift-p` | Command palette (`arc::ToggleCommandPalette`) |
| `cmd-b` / `cmd-alt-b` / `cmd-j` | Toggle file panel / env panel / response |
| `cmd-s` (request) | Save request |
| `cmd-enter` (request) | Send / stop request |
| `cmd-.` (request) | Cancel request |
| `cmd-l` (request) | Focus URL bar |
| `cmd-n` (files) | New file · `cmd-alt-n` new folder |
| `enter` / `f2` (files) | Rename · `backspace`/`delete` delete · `cmd-backspace` trash |
| `cmd-alt-c` (files) | Copy path |

Your overrides live in `~/.config/.arc/keymap.json` (seeded from
`initial-keymap.json` on first launch).
