# KeySlam

KeySlam is a keyboard and pointer playground for young children. Press keys to
fill every display with colorful letters, animals, and smiling shapes while the
app speaks their names; use the pointer to paint, make animated trails, and play
musical notes.

KeySlam began as a from-scratch Rust reimplementation of Scott Hanselman's
[BabySmash](https://github.com/shanselman/babysmash). It preserves that
project's delightful central idea while developing its own architecture,
identity, interaction modes, audio system, and visual effects. KeySlam is an
independent project and is not an official BabySmash release.

The implementation favors a small number of typed, reusable systems over a
platform-specific class hierarchy. All art and sounds are embedded in the
executable.

## Features

- Letters, top-row numbers, the full special-key animal map, and numpad shapes
- Pre-recorded English Opus speech, including distinct standalone and modifier
  performances for color names
- Matching defaults, letter grouping, fading, item limits, faces, spawn
  animation, and tap animation
- Speech, click audio, continuous stereo sinewave, and right-click piano
- Arrow/original blue hand pointers plus rainbow, fading-afterimage, neon-worm,
  sparkle, bubble, background-coloring, and piano-roll modes; the mouse wheel cycles
  effects and briefly pulses the hand larger or smaller
- One borderless, always-on-top game viewport per monitor (Windows uses native
  borderless fullscreen)
- Single-instance protection and Windows low-level protection for Windows keys,
  Alt+Tab, Alt+Escape, Ctrl+Escape, Print Screen, numpad reinterpretation, and
  related kiosk shortcuts
- External window-close requests are ignored; Alt+F4 remains the intentional
  way to exit
- Persistent settings

On Windows, the low-level keyboard guard runs as a hidden companion mode of
the same executable. It is scoped to KeySlam's foreground windows and exits
automatically with the game; no additional binary is installed.

The original application's self-updater is intentionally not copied. Release
and package updates should be handled by the platform package manager. Some
secure operating-system shortcuts (notably Ctrl+Alt+Delete) and desktop-level
touchpad gestures cannot be intercepted by an ordinary application. For an
OS-enforced lock that prevents the Windows shell from appearing at all, use
Windows Assigned Access with a dedicated account. On Linux, the window
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
| `Alt+F4` | Exit all KeySlam windows |
| Left-click / drag | Tap items, draw the pointer effect, and play pointer audio |
| Right-click | Play a chromatic or key-based major/minor piano note and show a fading ripple |
| Mouse wheel | Cycle pointer effects |

Coloring mode adds the 12-color palette and Clear screen button along the
bottom edge, plus a vertical brush-size slider on the left. Paint remains behind
letters, animals, and shapes. Clicks and drags use the same round brush, with a
live circular size preview under the custom cursor.

Piano-roll mode adds labeled keys down the left edge and extends each note's
click-zone boundary across the screen. The row under the pointer is highlighted,
and the available notes follow the configured Chromatic, Major, or Minor scale
and key used by right-click piano playback.

For a release executable:

```powershell
cargo build --release
```

The result is `target/release/keyslam.exe` on Windows.

## Custom voice recordings

KeySlam copies its English speech clips to an editable folder the first time it
runs. On Windows, paste this path into File Explorer:

```text
%APPDATA%\KeySlam\KeySlam\config\speech
```

Replace any `.opus` file with your own Ogg Opus recording, keeping the same
folder and filename, then restart KeySlam. Record only the word represented by
the file: `colors\standalone\red.opus` is the complete utterance “red,” while
`colors\modifier\red.opus` has the continuing delivery used before
`shapes\circle.opus`. Do not record a combined “red circle” clip. The `speech`
folder directly contains `animals`, `letters`, `numbers`, `colors`, and
`shapes`. Existing custom files are never overwritten.

Existing custom recordings from the former `common` and `en-EN` layouts are
copied into the flat folders without overwriting files already there.

To add alternate takes, append a number to the word's filename: `red1.opus`,
`red2.opus`, and so on. KeySlam discovers numbered takes in the same folder and
randomly chooses among the base file and all numbered versions each time the
word is spoken.

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

The automated suite checks the key map, numpad behavior, composed speech keys,
color cycling, delayed one-second fading, coloring controls, settings bounds,
item placement/caps, pointer pitch/pan mapping, piano range, and decoding of
every bundled sound.

## BabySmash roots, license, and attribution

KeySlam is MIT licensed. It owes its original concept and several parity
contracts and resources to Scott Hanselman and the contributors to BabySmash,
which is also MIT licensed. BabySmash resources retain their original
attribution; bundled animal artwork is from Microsoft Fluent Emoji. See
[LICENSE](LICENSE) and [THIRD-PARTY-NOTICES.md](THIRD-PARTY-NOTICES.md).

On first launch under the KeySlam name, existing settings and customized speech
from the former BabySmash Rust configuration directory are migrated when the
new KeySlam files do not already exist.
