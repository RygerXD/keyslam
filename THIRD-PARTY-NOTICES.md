# Third-party notices

KeySlam began as a clean Rust reimplementation of Scott Hanselman's
MIT-licensed [BabySmash](https://github.com/shanselman/babysmash) project.
KeySlam is now an independent project with its own name, architecture, and
features, while retaining clear credit for its roots. Bundled sound,
localization, icon, and emoji resources derived from that work are redistributed
under the notices that accompanied BabySmash.

## Microsoft Fluent Emoji

The Fluent artwork in `assets/packs/animals`, `assets/packs/foods`,
`assets/packs/instruments`, and `assets/packs/directions` comes from
[Microsoft Fluent Emoji](https://github.com/microsoft/fluentui-emoji) and is
licensed under the MIT License. Copyright Microsoft Corporation.

## Google Noto Emoji

The Android-style artwork in `assets/packs/animals`, `assets/packs/foods`,
`assets/packs/instruments`, and `assets/packs/directions` comes from
[Google Noto Emoji](https://github.com/googlefonts/noto-emoji). The image
resources are licensed under the Apache License 2.0. Copyright Google LLC.

## BabySmash resources

The remaining BabySmash resources and original application are licensed under
the MIT License. Copyright Scott Hanselman and contributors. See `LICENSE`.

## Synthetic speech and historical animal sounds

All current bundled audio is synthetic speech generated using Windows
System.Speech and Microsoft Zira Desktop. Text, voice settings, and file hashes
are recorded in `assets/public-audio.json`.

Earlier versions included Wikimedia Commons animal sounds. Those recordings
are no longer bundled. Their historical per-file attribution, source links,
license links, and modification details are retained in
[`assets/sounds/ANIMAL-SOUNDS.md`](assets/sounds/ANIMAL-SOUNDS.md).

## Shadertoy trail reference

The following pointer effects are Rust/egui adaptations of visual techniques
demonstrated by Shadertoy authors. The original shader source is not embedded:

- [Test 11 - fading mouse trails](https://www.shadertoy.com/view/mtSGDy) by gorplex
