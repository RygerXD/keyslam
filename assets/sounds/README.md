# Sound clips

Every spoken item has a folder containing one or more English Ogg Opus files.
This directory directly contains `animals`, `letters`, `numbers`, `colors`, and
`shapes`. Colors have two readings: `colors/standalone/<color>` for a color
spoken by itself and `colors/modifier/<color>` for the attributive reading
composed with a shape. Phrases such as "red circle" therefore retain natural
continuation intonation without needing a recording of every complete phrase.

At runtime the English clips are copied to the user's KeySlam configuration
directory without overwriting existing files. Add, replace, rename, or remove
files in an item's folder, then restart KeySlam. Filenames are unrestricted;
for example, `animals/tiger/RyanTigerGrowl.opus` is a valid tiger clip. Mono or
stereo Opus files are supported.

KeySlam sorts the `.opus` filenames alphabetically in each item folder and
cycles through them in that order, returning to the first after the last. The
cycle resets when the app restarts. Non-Opus files are ignored.

The clips in an animal folder may be recognizable animal sounds rather than
readings of its name. For example, `animals/dog/dog.opus` says "dog" and
`animals/dog/dog1.opus` barks, so both are part of the same ordered cycle. See
[`ANIMAL-SOUNDS.md`](ANIMAL-SOUNDS.md) for the sources and licenses of bundled
animal-sound takes.

On Windows the editable folder is:

`%APPDATA%\KeySlam\config\sounds`

KeySlam began as a Rust reimplementation of Scott Hanselman's BabySmash. The
sound-file organization grew from that compatibility work and is maintained
as part of KeySlam's independent audio system.
