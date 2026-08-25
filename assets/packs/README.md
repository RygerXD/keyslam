# Item packs

Each child folder is a pack shown dynamically in KeySlam's Settings dropdown.
Each item folder holds all PNG images and Ogg Opus sounds for that item. Files
of each type are sorted case-insensitively and cycled independently.

An optional `pack.json` controls key assignment. Its `items` array is ordered
as Escape, Space, then the stable extra-key order. Entries may be folder-name
strings or objects with `folder`, optional `name`, and optional `emoji` fields.
Without a manifest, KeySlam uses all item folders in alphabetical order.

Bundled PNG filenames identify their source set and Unicode sequence. The
Android images are from Google Noto Emoji and the Fluent images are from
Microsoft Fluent Emoji. `../images/extra-key-emoji.csv` records the pinned
source names and Unicode codepoints for food and instrument artwork. See
`THIRD-PARTY-NOTICES.md` and `../images/APACHE-2.0.txt` for licensing details.
