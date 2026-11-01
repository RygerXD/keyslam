# Sound clips

Every bundled spoken item has one synthetic English Ogg Opus clip. Common
speech lives in `letters/<letter>`, `numbers/<digit>`, `shapes/<shape>`, and
`colors/<standalone-or-modifier>/<color>` under this folder. Picture-item speech
lives beside its PNGs in `assets/packs/<pack>/<item>`.

The public clips use Windows System.Speech with Microsoft Zira Desktop. The
script `tools/generate_starter_speech.ps1` regenerates them, and
`assets/public-audio.json` records text, voice settings, and SHA-256 hashes.
`tools/check_public_assets.py` checks that every item has exactly one approved
clip and that no other recordings are included. Playback works offline.

Keep personal voices and Family packs in a complete asset folder outside the
repository. Configure its absolute path using the ignored `.keyslam-assets-path`
file or `KEYSLAM_ASSETS_DIR`; see the main README. The Settings asset button opens
that configured folder. Built-in installation preserves existing files and skips
adding synthetic clips to an item that already has personal Opus recordings.

Put any number of mono or stereo Opus clips in a personal item's folder. KeySlam
sorts their filenames alphabetically and cycles through them, returning to the
first after the last. The cycle resets on restart, and non-Opus files are ignored.
Restart after editing sounds because decoded clips are cached.

Record only the represented word: a standalone red clip says "red," a modifier
red clip precedes a separate shape clip, and a circle clip says "circle." Do not
record an entire "red circle" phrase in either folder.

`ANIMAL-SOUNDS.md` retains attribution for recordings distributed in earlier
versions. Those recordings are no longer part of the public asset bundle.
