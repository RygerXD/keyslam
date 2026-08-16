# Sound clips

KeySlam uses one English Ogg Opus file per spoken word or recognizable animal
sound. This directory directly contains `animals`, `letters`, `numbers`,
`colors`, and `shapes`. Colors have
two readings: `colors/standalone` for a color spoken by itself and
`colors/modifier` for the attributive reading composed with a shape. Phrases
such as "red circle" therefore retain natural continuation intonation without
needing a recording of every complete phrase.

At runtime the English clips are copied to the user's KeySlam configuration
directory without overwriting existing files. Replace any file there with
another Ogg Opus recording of the same word and performance, then restart
KeySlam. Keep the same directory and filename. Mono or stereo Opus files are
supported.

Additional takes use a numeric suffix: `red1.opus`, `red2.opus`, and so on.
KeySlam randomly selects from the base clip and every numbered take in that
same directory each time it speaks the word.

The numbered takes in `animals` may be the animal's recognizable sound rather
than another reading of its name. For example, `dog.opus` says "dog" and
`dog1.opus` barks, so both are part of the same randomized learning pool. See
[`ANIMAL-SOUNDS.md`](ANIMAL-SOUNDS.md) for the sources and licenses of bundled
animal-sound takes.

On Windows the editable folder is:

`%APPDATA%\KeySlam\config\sounds`

KeySlam began as a Rust reimplementation of Scott Hanselman's BabySmash. The
sound-file organization grew from that compatibility work and is maintained
as part of KeySlam's independent audio system.
