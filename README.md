# KeySlam

KeySlam is a keyboard and pointer playground for young children. Press keys to
fill every display with colorful letters, pictures, and smiling shapes while the
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

- Letters, top-row numbers, numpad shapes, and selectable Animals, Foods, or
  Instruments picture sets for every other key
- Pre-recorded English Opus speech, including distinct standalone and modifier
  performances for color names
- Matching defaults, letter grouping, fading, item limits, faces, spawn
  animation, and tap animation
- Speech, continuous stereo sinewave, and configurable paint-color and right-click audio
- Original blue hand pointer plus rainbow, fading-afterimage, neon-worm,
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
| Any key | Show and announce its letter, selected picture, or shape |
| `Alt+O` | Open settings |
| `Alt+F4` | Exit all KeySlam windows |
| Left-click / drag | Tap items, draw the pointer effect, and play pointer audio |
| Right-click | Play the configured sound and show a fading ripple |
| Mouse wheel | Cycle pointer effects |

Coloring mode adds the 12-color palette and Clear screen button along the
bottom edge, plus a vertical brush-size slider on the left. Paint remains behind
letters, pictures, and shapes. Clicks and drags use the same round brush, with a
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

KeySlam copies its English sound clips to an editable folder the first time it
runs. On Windows, paste this path into File Explorer:

```text
%APPDATA%\KeySlam\config\sounds
```

Each item has its own folder. Put any Ogg Opus recordings for that item inside
it, using any filenames you like, then restart KeySlam. For example,
`animals\tiger\RyanTigerGrowl.opus` is a valid tiger clip. Record only the word
represented by a speech folder: clips in `colors\standalone\red` are complete
utterances of “red,” while clips in `colors\modifier\red` have the continuing
delivery used before a clip from `shapes\circle`. Do not record a combined “red
circle” clip. The `sounds` folder directly contains `animals`, `foods`,
`instruments`, `letters`, `numbers`, `colors`, and `shapes`. Existing custom
files are never overwritten.

Existing custom recordings from the former `common` and `en-EN` layouts are
copied into the item folders without overwriting files already there. Clips
from the previous flat filename layout are migrated the same way.

KeySlam sorts the `.opus` filenames alphabetically in each item folder and
cycles through them in that order, returning to the first clip after the last.
The cycle starts over when KeySlam restarts. Non-Opus files are ignored.

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
| `src/responses.rs` | Deterministic key-to-glyph/picture/shape contract |
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
attribution; bundled animal artwork is from Google Noto Emoji and Microsoft
Fluent Emoji. See
[LICENSE](LICENSE) and [THIRD-PARTY-NOTICES.md](THIRD-PARTY-NOTICES.md).

Existing settings and customized sounds from the former doubled KeySlam
configuration directory are migrated when the new KeySlam files do not already
exist.
