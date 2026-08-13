# Speech clips

KeySlam uses one Ogg Opus file per spoken word. `common` contains shared
letters, digits, and animal names; locale folders contain translated color and
shape names plus any pronunciation overrides. Color and shape clips are played
in the locale's configured word order, so phrases such as "red circle" do not
need their own recording.

At runtime the clips for the active locale are copied to the user's KeySlam
configuration directory without overwriting existing files. Replace any file
there with another Ogg Opus recording of the same word, then restart KeySlam.
Keep the same directory and filename. Mono or stereo Opus files are supported.

On Windows the editable folder is:

`%APPDATA%\KeySlam\KeySlam\config\speech`

KeySlam began as a Rust reimplementation of Scott Hanselman's BabySmash. The
speech-file organization grew from that compatibility work and is maintained
as part of KeySlam's independent audio system.
