# BabySmash Rust

A from-scratch Rust reimplementation of
[BabySmash](https://github.com/RygerXD/babysmash): press keys to fill every
display with colorful letters, animals, and smiling shapes while the app speaks
their names or plays laughter.

The implementation favors a small number of typed, reusable systems over a
platform-specific class hierarchy. All art, sounds, and translations are
embedded in the executable.

## Parity

- Letters, top-row numbers, the full special-key animal map, and numpad shapes
- Localized color/shape speech for English, German, Greek, Spanish, French,
  Latvian, Portuguese, and Russian
- Matching defaults, letter grouping, fading, item limits, faces, spawn
  animation, and tap animation
- Speech, laughter, startup audio, click audio, continuous stereo sinewave, and
  right-click piano
- Arrow/hand pointers plus rainbow, sparkle, and bubble trails; the mouse wheel
  cycles effects
- One borderless, always-on-top game viewport per monitor
- Single-instance protection and Windows low-level protection for Windows keys,
  Alt+Tab, Ctrl+Escape, Print Screen, numpad reinterpretation, and related kiosk
  shortcuts
- Persistent settings and the `--fps` diagnostic overlay

The original application's self-updater is intentionally not copied. Release
and package updates should be handled by the platform package manager. Some
secure operating-system shortcuts (notably Ctrl+Alt+Delete and desktop-level
touchpad gestures) cannot be intercepted by an ordinary application. On Linux,
the window consumes keys it receives, but the desktop compositor may retain its
own global shortcuts.

## Run

Install the current stable Rust toolchain (Rust 1.92 or newer), then:

```powershell
cargo run --release
```

Useful development modes:

```powershell
# Regular resizable window; avoids taking over every monitor.
cargo run -- --windowed

# Show frame rate and retained item count.
cargo run --release -- --fps
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

For a release executable:

```powershell
cargo build --release
```

The result is `target/release/babysmash-rs.exe` on Windows.

## Stability and DRY design

- `Settings` is one validated, typed schema; writes use an atomic same-directory
  replacement, so interruption cannot leave a half-written JSON file.
- `Game` owns the canonical figure queue. Every monitor renders that state with
  its own placement and pointer state, avoiding duplicate gameplay logic.
- Audio uses one bounded, nonblocking mixer. It resamples bundled PCM and legacy
  MP3-in-WAVE assets, caps simultaneous voices, and keeps synthesis off the UI
  thread.
- Speech runs on a dedicated worker. Missing speech/audio devices produce a
  visible warning while the visual game continues.
- Figures and cursor particles have hard bounds, eliminating unbounded growth
  under key smashing or pointer movement.
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
| `src/speech.rs` | Platform speech worker |
| `src/settings.rs` | Typed defaults, validation, atomic persistence |
| `src/platform.rs` | Windows kiosk keyboard guard |

## Verify

```powershell
cargo fmt -- --check
cargo test
cargo clippy --all-targets -- -D warnings
```

The automated suite checks the key map, numpad behavior, localized word order,
settings bounds, item cap, pointer pitch/pan mapping, piano range, and decoding
of every bundled sound.

## License and attribution

MIT licensed. BabySmash resources retain their original attribution; bundled
animal artwork is from Microsoft Fluent Emoji. See [LICENSE](LICENSE) and
[THIRD-PARTY-NOTICES.md](THIRD-PARTY-NOTICES.md).

