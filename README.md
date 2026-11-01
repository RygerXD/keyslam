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
- Arrow keys and RedOctane USB soft-mat arrows show matching arrow pictures
  and speak “Up,” “Down,” “Left,” or “Right”
- Synthetic English Opus speech for every built-in item, including standalone
  and modifier readings for color names; personal recordings can be kept locally
- Matching defaults, letter grouping, fading, item limits, faces, spawn
  animation, and tap animation
- Speech, continuous stereo sinewave, and configurable paint-color and right-click audio
- Original blue hand pointer plus rainbow, fading-afterimage,
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

If a Windows laptop's three-finger swipe accidentally switches apps or exits
the game, disable the system gesture in **Settings → Bluetooth & devices →
Touchpad**: set **Three-finger gestures → Swipes** to **Nothing**, and repeat
the same change for **Four-finger gestures**.

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
| Arrow keys / dance mat arrows | Show and announce Up, Down, Left, or Right |
| `Alt+O` | Open settings |
| `Alt+F4` | Exit all KeySlam windows |
| Left-click / drag | Tap items, draw the pointer effect, and play pointer audio |
| Right-click | Play the configured sound and show a fading ripple |
| Mouse wheel | Cycle pointer effects |

On Windows, the ten-button RedOctane USB soft mat is detected automatically,
including when plugged in after KeySlam starts. Each panel triggers once per
press; lift your foot before stepping again. Multiple panels can be pressed
together, including opposite arrows. The other six panels use the selected
picture pack (F1–F4, Escape, and Space). Mat input is active while a KeySlam
window has focus and settings are closed. No keyboard mapper is needed.

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

Release builds use full link-time optimization and a single code generation unit
to reduce executable size while retaining speed optimizations. This makes release
compilation slower; regular development builds are unaffected. The WAV decoder
is a development-only dependency for `examples/encode_speech.rs`.

## Editable assets

Keep `keyslam.exe` in a folder named `keyslam` on the Desktop. The first time
it runs, KeySlam copies its built-in game assets to an editable folder beside
the executable:

```text
Desktop\keyslam\assets
```

Settings are saved beside the executable at
`Desktop\keyslam\settings\settings.json`. KeySlam does not create settings or
asset folders in AppData or elsewhere on the computer.

By default, editable game assets are under `assets` beside `keyslam.exe`. The
Settings button opens this folder directly. KeySlam installs any missing built-in files
without overwriting existing files, so end-user edits are preserved.

Each pack is a folder under `assets\packs`, and each item is a folder inside its
pack. PNG images and Ogg Opus recordings for one item live together. For
example, `assets\packs\animals\tiger\` holds every tiger image and sound. Add a
new pack folder and it appears in the Settings dropdown without a code change
or restart.

By default, item folders are assigned to extra keys in alphabetical order. Add
a `pack.json` file to assign item folders to named keys directly:

```json
{
  "keys": [
    { "key": "Escape", "item": "tiger" },
    { "key": "Space", "item": "bear" },
    { "key": "F1", "item": "red panda" }
  ]
}
```

The bundled manifests list every assignable key using readable names such as
`Print Screen`, `Left Shift`, and `Page Up`; use one as a template. Change an
entry's `item` to the exact name of another item folder to reassign that key.
Key names are case-insensitive. Optional `name` and `emoji` fields override the
spoken/display name and the fallback shown when no valid PNG exists. A key
omitted from the manifest displays the generic star item. Numpad digits and
Decimal, Multiply, and Add always produce KeySlam's built-in shapes.
The four arrow keys always use the built-in `directions` pack, regardless of
the selected pack or its manifest. Its PNGs and Opus recordings are editable
in `assets\packs\directions` just like animal pictures and voices. Directions
is reserved for the keyboard and dance-pad arrows and does not appear in the
set selector. Arrows keep their orientation during animations.

Development builds read the repository `assets` folder directly. Packaged
builds read `assets` beside `keyslam.exe`, unless a personal asset root is configured.

### Personal assets and public builds

The repository contains Animals, Foods, Instruments, and the built-in Directions
assets. Every public audio clip is synthetic speech generated with the installed
Windows voice Microsoft Zira Desktop. Family packs, personal voices, and raw
recordings belong in a separate folder outside this checkout.

To keep personal assets, copy the full `assets` folder outside the repository,
then put its absolute path on one line in `.keyslam-assets-path`. For development,
this ignored file goes in the repository root. For release executables, it goes
beside `keyslam.exe`. Alternatively, set `KEYSLAM_ASSETS_DIR` to that absolute
path; the environment variable takes priority. The Settings asset button opens
the configured folder. This setting is read at runtime and is never embedded.

Only the repository `assets` folder is embedded in builds, even when development
runs use a personal folder. Missing built-in assets are installed into the
configured folder. An item that already has Opus recordings keeps its existing
voice cycle without adding synthetic takes. Add custom packs and recordings to
the personal folder, and back that folder up separately from Git.

Regenerate the public voice on Windows with:

```powershell
powershell -NoProfile -File tools/generate_starter_speech.ps1 -ReplaceExisting
python tools/check_public_assets.py
```

`-Voice` selects another installed System.Speech voice and `-Rate` controls its
speed. Generation stages WAV/Opus files under ignored `target`, preserves replaced
takes there, and records the generated text, voice, and SHA-256 in
`assets/public-audio.json`. Its generated `assets/.gitignore` allows only inventoried
audio filenames to be newly tracked. CI checks the inventory, complete item coverage, and
the allowed public packs. No speech service credentials or network access are
needed for generation or playback.

Removing a recording from the current files does not remove earlier Git commits
or published releases containing it. Cleaning those copies requires a separate
history/release cleanup; `.gitignore` cannot erase existing history.

Put any number of PNG or Opus files in an item folder; KeySlam sorts each type
alphabetically and cycles through it. New filenames are picked up while the app
is running. Record only the word represented by a speech folder: clips in
`assets\sounds\colors\standalone\red` are complete utterances of “red,” while
clips in `assets\sounds\colors\modifier\red` have the continuing delivery used
before a clip from `assets\sounds\shapes\circle`. Do not record a combined “red
circle” clip. `assets\sounds` contains letters, numbers, colors, and shapes.
Existing custom files are never overwritten.

The editable SVGs in `assets\shapes` control the built-in shapes and faces, and
`assets\keyslam.png` controls the app icon. Restart KeySlam after editing a
sound, SVG, or icon because those files are loaded or cached at startup. Pack
images and manifests are discovered while the app is running.

The image and sound cycles start over when KeySlam restarts. Other file types
are ignored.

## Stability and DRY design

- `Settings` is one validated, typed schema; writes use an atomic same-directory
  replacement, so interruption cannot leave a half-written JSON file.
- `Game` owns the canonical figure queue. Every monitor renders that state with
  its own placement and pointer state, avoiding duplicate gameplay logic.
- Audio uses one bounded, nonblocking mixer. It resamples effects, decodes
  pre-recorded Opus speech, and caps simultaneous voices.
- Piano samples are stored once in mono and mixed to stereo. Voice cleanup and
  sine pan calculations run once per audio buffer.
- Display sizing and piano key boundaries are recalculated only when their
  inputs change. The image texture cache retains at most 128 recently used
  entries; invalid images are retried at most once per second while cached.
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
| `src/assets.rs` | Editable asset root and non-overwriting starter installation |
| `src/game.rs` | Canonical game state, grouping, placement, animation state |
| `src/render.rs` | Shapes, faces, emoji, text, pointers, particles |
| `src/responses.rs` | Deterministic key-to-glyph/picture/shape contract |
| `src/audio.rs` | Embedded sound decoding, mixer, sinewave, piano |
| `src/settings.rs` | Typed defaults, validation, atomic persistence |
| `src/platform.rs` | Windows kiosk keyboard guard |
| `src/dance_mat.rs` | Windows RedOctane soft-mat detection and button input |

## Verify

```powershell
python tools/check_public_assets.py
cargo fmt -- --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
```

## BabySmash roots, license, and attribution

KeySlam is MIT licensed. It owes its original concept and several parity
contracts and resources to Scott Hanselman and the contributors to BabySmash,
which is also MIT licensed. BabySmash resources retain their original
attribution; bundled animal artwork is from Google Noto Emoji and Microsoft
Fluent Emoji. See
[LICENSE](LICENSE) and [THIRD-PARTY-NOTICES.md](THIRD-PARTY-NOTICES.md).
