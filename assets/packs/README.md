# Item packs

Each child folder is a pack shown dynamically in KeySlam's Settings dropdown.
Each item folder holds all PNG images and Ogg Opus sounds for that item. Files
of each type are sorted case-insensitively and cycled independently.

An optional `pack.json` controls key assignment directly. Each entry names the
physical `key` and the item folder assigned to it, so changing an assignment is
just changing its `item` value:

```json
{
  "keys": [
    { "key": "Escape", "item": "bear" },
    { "key": "Space", "item": "tiger" },
    { "key": "F1", "item": "bee" }
  ]
}
```

Use the key names already present in a bundled manifest as a complete template.
Key names are case-insensitive. An entry may also have optional `name` and
`emoji` fields to override the spoken/display name and image fallback. When a
key is absent from `keys`, that key displays the generic star item. Without a
manifest, KeySlam assigns item folders alphabetically.

Numpad digits and Decimal, Multiply, and Add remain KeySlam's built-in shapes,
so pack manifests do not assign items to those keys.

When developing with `cargo run`, KeySlam reads this `assets/packs` folder
directly, including manifest edits made while the app is running. Packaged
release builds read the editable `packs` folder beside `keyslam.exe`.

Bundled PNG filenames identify their source set and Unicode sequence. The
Android images are from Google Noto Emoji and the Fluent images are from
Microsoft Fluent Emoji. `../images/extra-key-emoji.csv` records the pinned
source names and Unicode codepoints for food and instrument artwork. See
`THIRD-PARTY-NOTICES.md` and `../images/APACHE-2.0.txt` for licensing details.
