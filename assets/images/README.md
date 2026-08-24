# Extra-key images

Artwork lives in `animals/<item>/`, `foods/<item>/`, or `instruments/<item>/`.
Every PNG in an item folder is available to the game, regardless of its
filename. KeySlam sorts filenames case-insensitively and rotates through them
in order each time that item is created, wrapping after the last image.

Bundled filenames identify both their source set and Unicode sequence. For
example, `animals/tiger/android-1f42f.png` and
`animals/tiger/fluent-1f42f.png` are two tiger images in the same cycle.

The bundled Android images are from Google Noto Emoji. The bundled Fluent
images are from Microsoft Fluent Emoji. `extra-key-emoji.csv` records the
source name and Unicode codepoint for every food and instrument. The matching
download script pins the exact upstream revisions. See the repository's
`THIRD-PARTY-NOTICES.md` for source details and `APACHE-2.0.txt` for the Noto
image-resource license.
