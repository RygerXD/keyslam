# BabySmash Rust

A from-scratch Rust reimplementation of
[BabySmash](https://github.com/RygerXD/babysmash): press keys to fill every
display with colorful letters, animals, and smiling shapes while the app speaks
their names.

The implementation favors a small number of typed, reusable systems over a
platform-specific class hierarchy. All art, sounds, and translations are
embedded in the executable.

## Parity

- Letters, top-row numbers, the full special-key animal map, and numpad shapes
- Pre-recorded Opus speech for English, German, Greek, Spanish, French,
  Latvian, Portuguese, and Russian
- Matching defaults, letter grouping, fading, item limits, faces, spawn
  animation, and tap animation
- Speech, click audio, continuous stereo sinewave, and right-click piano
- Arrow/original blue hand pointers plus rainbow, fading-afterimage, neon-worm,
  sparkle, bubble, and background-coloring modes; the mouse wheel cycles
  effects and briefly pulses the hand larger or smaller
- One borderless, always-on-top game viewport per monitor (Windows uses native
  borderless fullscreen)
- Single-instance protection and Windows low-level protection for Windows keys,
  Alt+Tab, Alt+Escape, Ctrl+Escape, Print Screen, numpad reinterpretation, and
  related kiosk shortcuts
- Windows kiosk focus recovery that restores a minimized or deactivated game
  after Task View, Show Desktop, or app-switching touchpad gestures
- External window-close requests are ignored; Alt+F4 remains the intentional
  way to exit
- Persistent settings

The original application's self-updater is intentionally not copied. Release
and package updates should be handled by the platform package manager. Some
secure operating-system shortcuts (notably Ctrl+Alt+Delete) and the gesture
itself cannot be intercepted by an ordinary application. BabySmash responds to
desktop-level touchpad gestures by immediately reclaiming its fullscreen focus.
For an OS-enforced lock that prevents the Windows shell from appearing at all,
use Windows Assigned Access with a dedicated account. On Linux, the window
consumes keys it receives, but the desktop compositor may retain its own global
shortcuts.

## Run

Install the current stable Rust toolchain (Rust 1.92 or newer), then:

```powershell
cargo run --release
```

Useful development modes:

```powershell
# Regular resizable window; avoids taking over every monitor.
cargo run -- --windowed

```

Controls:

| Input | Action |
|---|---|
| Any key | Show and announce its letter, animal, or shape |
| `Alt+O` | Open settings |
| `Alt+F4` | Exit all BabySmash windows |
| Left-click / drag | Tap items, draw the pointer effect, and play pointer audio |
| Right-click | Play a piano note based on vertical position |
| Mouse wheel | Cycle pointer effects |

Coloring mode adds the 12-color palette and Clear screen button along the
bottom edge, plus a vertical brush-size slider on the left. Paint remains behind
letters, animals, and shapes. Clicks and drags use the same round brush, with a
live circular size preview under the custom cursor.

For a release executable:

```powershell
cargo build --release
```

The result is `target/release/babysmash-rs.exe` on Windows.

## Custom voice recordings

BabySmash copies the active language's speech clips to an editable folder the
first time it runs. On Windows, paste this path into File Explorer:

```text
%APPDATA%\BabySmash\BabySmash Rust\config\speech
```

Replace any `.opus` file with your own Ogg Opus recording, keeping the same
folder and filename, then restart BabySmash. Record only the word represented by
the file: for example, replace `en-EN\colors\red.opus` and
`en-EN\shapes\circle.opus`, not a combined "red circle" recording. Files in
`common` contain letters, digits, and animal names shared by languages unless a
locale has an explicit pronunciation override (such as Canadian English “Zed”).
Existing custom files are never overwritten.

## Stability and DRY design

- `Settings` is one validated, typed schema; writes use an atomic same-directory
  replacement, so interruption cannot leave a half-written JSON file.
- `Game` owns the canonical figure queue. Every monitor renders that state with
  its own placement and pointer state, avoiding duplicate gameplay logic.
- Audio uses one bounded, nonblocking mixer. It resamples effects, decodes
  pre-recorded Opus speech, and caps simultaneous voices.
- Missing or invalid speech clips produce a warning in settings while the
  visual game continues.
- Active figures and cursor particles are bounded. Figures displaced by the
  item-count limit finish the same one-second fade used by timed removal.
- The Windows hook is held by an RAII guard, so it is detached automatically on
  shutdown or unwinding.

Source layout:

| File | Responsibility |
|---|---|
| `src/app.rs` | Windows/viewports, input routing, settings UI |
| `src/game.rs` | Canonical game state, grouping, placement, animation state |
| `src/render.rs` | Shapes, faces, emoji, text, pointers, particles |
| `src/responses.rs` | Deterministic key-to-glyph/animal/shape contract |
| `src/audio.rs` | Embedded sound decoding, mixer, sinewave, piano |
| `src/settings.rs` | Typed defaults, validation, atomic persistence |
| `src/platform.rs` | Windows kiosk keyboard guard |

## Verify

```powershell
cargo fmt -- --check
cargo test
cargo clippy --all-targets -- -D warnings
```

The automated suite checks the key map, numpad behavior, localized word order,
color cycling, delayed one-second fading, coloring controls, settings bounds,
item placement/caps, pointer pitch/pan mapping, piano range, and decoding of
every bundled sound.

## License and attribution

MIT licensed. BabySmash resources retain their original attribution; bundled
animal artwork is from Microsoft Fluent Emoji. See [LICENSE](LICENSE) and
[THIRD-PARTY-NOTICES.md](THIRD-PARTY-NOTICES.md).
