# Animal images

Animal artwork lives in `animals/<animal>/`. Every PNG in an animal folder is
available to the game, regardless of its filename. KeySlam sorts the filenames
case-insensitively and rotates through them in order each time that animal is
created, wrapping back to the first image after the last.

Bundled filenames identify both their source set and Unicode sequence. For
example, `animals/tiger/android-1f42f.png` and
`animals/tiger/fluent-1f42f.png` are two tiger images in the same cycle.

The bundled Android images are from Google Noto Emoji. The bundled Fluent
images are from Microsoft Fluent Emoji. See the repository's
`THIRD-PARTY-NOTICES.md` for source details and `APACHE-2.0.txt` for the Noto
image-resource license.
