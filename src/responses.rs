use crate::packs::{self, PackItem};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShapeKind {
    Star,
    Cross,
    Heart,
    Oval,
    Rectangle,
    Triangle,
    Square,
    Pentagon,
    Hexagon,
    Septagon,
    Octagon,
    Trapezoid,
    Circle,
}

impl ShapeKind {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Star => "Star",
            Self::Cross => "Cross",
            Self::Heart => "Heart",
            Self::Oval => "Oval",
            Self::Rectangle => "Rectangle",
            Self::Triangle => "Triangle",
            Self::Square => "Square",
            Self::Pentagon => "Pentagon",
            Self::Hexagon => "Hexagon",
            Self::Septagon => "Septagon",
            Self::Octagon => "Octagon",
            Self::Trapezoid => "Trapezoid",
            Self::Circle => "Circle",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResponseKind {
    Glyph(char),
    Emoji(String),
    Shape(ShapeKind),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyResponse {
    pub kind: ResponseKind,
    pub spoken_text: String,
    pub extra_key_set: Option<String>,
    pub item_folder: Option<String>,
}

impl KeyResponse {
    fn extra(item: PackItem, set: &str) -> Self {
        Self {
            kind: ResponseKind::Emoji(item.fallback_emoji),
            spoken_text: item.label,
            extra_key_set: Some(set.to_owned()),
            item_folder: Some(item.folder),
        }
    }

    fn shape(shape: ShapeKind) -> Self {
        Self {
            kind: ResponseKind::Shape(shape),
            spoken_text: shape.name().to_owned(),
            extra_key_set: None,
            item_folder: None,
        }
    }
}

const OTHER_KEYS: [(&str, &str); 67] = [
    ("F1", "F1"),
    ("F2", "F2"),
    ("F3", "F3"),
    ("F4", "F4"),
    ("F5", "F5"),
    ("F6", "F6"),
    ("F7", "F7"),
    ("F8", "F8"),
    ("F9", "F9"),
    ("F10", "F10"),
    ("F11", "F11"),
    ("F12", "F12"),
    ("Snapshot", "Print Screen"),
    ("Scroll", "Scroll Lock"),
    ("Pause", "Pause"),
    ("Oem3", "Backtick"),
    ("OemMinus", "Minus"),
    ("OemPlus", "Equals"),
    ("Back", "Backspace"),
    ("Tab", "Tab"),
    ("Oem4", "Left Bracket"),
    ("Oem6", "Right Bracket"),
    ("Oem5", "Backslash"),
    ("Capital", "Caps Lock"),
    ("Oem1", "Semicolon"),
    ("Oem7", "Quote"),
    ("Return", "Enter"),
    ("LeftShift", "Left Shift"),
    ("OemComma", "Comma"),
    ("OemPeriod", "Period"),
    ("Oem2", "Slash"),
    ("RightShift", "Right Shift"),
    ("LeftCtrl", "Left Ctrl"),
    ("LWin", "Left Windows"),
    ("LeftAlt", "Left Alt"),
    ("RightAlt", "Right Alt"),
    ("RWin", "Right Windows"),
    ("Apps", "Menu"),
    ("RightCtrl", "Right Ctrl"),
    ("Insert", "Insert"),
    ("Home", "Home"),
    ("Prior", "Page Up"),
    ("Delete", "Delete"),
    ("End", "End"),
    ("Next", "Page Down"),
    ("Left", "Left Arrow"),
    ("Up", "Up Arrow"),
    ("Down", "Down Arrow"),
    ("Right", "Right Arrow"),
    ("NumLock", "Num Lock"),
    ("Divide", "Numpad Divide"),
    ("Multiply", "Numpad Multiply"),
    ("Subtract", "Numpad Subtract"),
    ("Add", "Numpad Add"),
    ("Decimal", "Numpad Decimal"),
    ("F13", "F13"),
    ("F14", "F14"),
    ("F15", "F15"),
    ("F16", "F16"),
    ("F17", "F17"),
    ("F18", "F18"),
    ("F19", "F19"),
    ("F20", "F20"),
    ("F21", "F21"),
    ("F22", "F22"),
    ("F23", "F23"),
    ("F24", "F24"),
];

pub fn response_for_set(key_name: &str, extra_key_set: &str) -> KeyResponse {
    // Directions are built in, independent of the selected picture pack.
    let direction = match key_name {
        "Up" | "ArrowUp" | "Up Arrow" => Some(("up", "Up", "⬆️")),
        "Down" | "ArrowDown" | "Down Arrow" => Some(("down", "Down", "⬇️")),
        "Left" | "ArrowLeft" | "Left Arrow" => Some(("left", "Left", "⬅️")),
        "Right" | "ArrowRight" | "Right Arrow" => Some(("right", "Right", "➡️")),
        _ => None,
    };
    if let Some((folder, label, emoji)) = direction {
        return KeyResponse::extra(
            PackItem {
                folder: folder.to_owned(),
                label: label.to_owned(),
                fallback_emoji: emoji.to_owned(),
            },
            "directions",
        );
    }
    match key_name {
        "Decimal" | "NumpadDecimal" => return KeyResponse::shape(ShapeKind::Circle),
        "NumPad0" => return KeyResponse::shape(ShapeKind::Oval),
        "NumPad1" => return KeyResponse::shape(ShapeKind::Rectangle),
        "NumPad2" => return KeyResponse::shape(ShapeKind::Heart),
        "NumPad3" => return KeyResponse::shape(ShapeKind::Triangle),
        "NumPad4" => return KeyResponse::shape(ShapeKind::Square),
        "NumPad5" => return KeyResponse::shape(ShapeKind::Pentagon),
        "NumPad6" => return KeyResponse::shape(ShapeKind::Hexagon),
        "NumPad7" => return KeyResponse::shape(ShapeKind::Septagon),
        "NumPad8" => return KeyResponse::shape(ShapeKind::Octagon),
        "NumPad9" => return KeyResponse::shape(ShapeKind::Trapezoid),
        "Multiply" | "NumpadMultiply" | "Asterisk" | "*" => {
            return KeyResponse::shape(ShapeKind::Star);
        }
        "Add" | "NumpadAdd" => return KeyResponse::shape(ShapeKind::Cross),
        "Escape" => return extra_response(extra_key_set, "Escape", 0),
        "Space" => return extra_response(extra_key_set, "Space", 1),
        _ => {}
    }

    let normalized = normalize_key_name(key_name);
    if let Some((index, (_, manifest_key))) = OTHER_KEYS
        .iter()
        .enumerate()
        .find(|(_, (input_key, _))| *input_key == normalized)
    {
        return extra_response(extra_key_set, manifest_key, index + 2);
    }

    let bytes = normalized.as_bytes();
    if bytes.len() == 1 && bytes[0].is_ascii_alphabetic() {
        let glyph = bytes[0].to_ascii_uppercase() as char;
        return KeyResponse {
            kind: ResponseKind::Glyph(glyph),
            spoken_text: String::new(),
            extra_key_set: None,
            item_folder: None,
        };
    }
    if bytes.len() == 2 && bytes[0] == b'D' && bytes[1].is_ascii_digit() {
        return KeyResponse {
            kind: ResponseKind::Glyph(bytes[1] as char),
            spoken_text: String::new(),
            extra_key_set: None,
            item_folder: None,
        };
    }

    extra_response(extra_key_set, normalized, stable_hash(normalized) as usize)
}

fn extra_response(set: &str, key: &str, legacy_index: usize) -> KeyResponse {
    let item = packs::item(set, key, legacy_index).unwrap_or(PackItem {
        folder: "item".to_owned(),
        label: "Item".to_owned(),
        fallback_emoji: "★".to_owned(),
    });
    KeyResponse::extra(item, set)
}

fn normalize_key_name(key_name: &str) -> &str {
    match key_name {
        "Enter" => "Return",
        "CapsLock" => "Capital",
        "PrintScreen" => "Snapshot",
        "PageUp" => "Prior",
        "PageDown" => "Next",
        "Backspace" => "Back",
        "ShiftLeft" => "LeftShift",
        "ShiftRight" => "RightShift",
        "ControlLeft" => "LeftCtrl",
        "ControlRight" => "RightCtrl",
        "AltLeft" => "LeftAlt",
        "AltRight" => "RightAlt",
        "SuperLeft" => "LWin",
        "SuperRight" => "RWin",
        "Backtick" => "Oem3",
        "Semicolon" | "Colon" => "Oem1",
        "Slash" | "Questionmark" => "Oem2",
        "OpenBracket" => "Oem4",
        "Backslash" | "Pipe" | "IntlBackslash" => "Oem5",
        "CloseBracket" => "Oem6",
        "Quote" => "Oem7",
        "Minus" => "OemMinus",
        "Plus" | "Equals" => "OemPlus",
        "Comma" => "OemComma",
        "Period" => "OemPeriod",
        "0" => "D0",
        "1" => "D1",
        "2" => "D2",
        "3" => "D3",
        "4" => "D4",
        "5" => "D5",
        "6" => "D6",
        "7" => "D7",
        "8" => "D8",
        "9" => "D9",
        _ => key_name,
    }
}

fn stable_hash(value: &str) -> u32 {
    value.chars().fold(2_166_136_261_u32, |hash, character| {
        (hash ^ character.to_ascii_uppercase() as u32).wrapping_mul(16_777_619)
    })
}

#[cfg(test)]
mod tests {
    use super::{ResponseKind, ShapeKind, response_for_set};

    #[test]
    fn arrows_keep_their_direction_across_picture_packs_and_key_names() {
        for pack in ["animals", "foods", "instruments", "family", "custom-pack"] {
            for (name, emoji) in [
                ("Up", "⬆️"),
                ("Down", "⬇️"),
                ("Left", "⬅️"),
                ("Right", "➡️"),
            ] {
                for key in [
                    name.to_owned(),
                    format!("Arrow{name}"),
                    format!("{name} Arrow"),
                ] {
                    let response = response_for_set(&key, pack);
                    assert_eq!(response.kind, ResponseKind::Emoji(emoji.to_owned()));
                    assert_eq!(response.spoken_text, name);
                    assert_eq!(response.extra_key_set.as_deref(), Some("directions"));
                    assert_eq!(response.item_folder, Some(name.to_ascii_lowercase()));
                }
            }
        }
        // Keyboard numpad directions continue to produce their original shapes.
        assert_eq!(
            response_for_set("NumPad8", "animals").kind,
            ResponseKind::Shape(ShapeKind::Octagon)
        );
        assert_eq!(
            response_for_set("A", "animals").kind,
            ResponseKind::Glyph('A')
        );
    }

    #[test]
    fn direction_assets_have_decodable_pictures_and_audible_speech()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/packs/directions");
        for (name, codepoint) in [
            ("up", "2b06"),
            ("down", "2b07"),
            ("left", "2b05"),
            ("right", "27a1"),
        ] {
            let folder = root.join(name);
            for style in ["android", "fluent"] {
                let picture = image::open(folder.join(format!("{style}-{codepoint}.png")))?;
                assert!(picture.width() >= 128 && picture.height() >= 128);
            }
            let bytes = std::fs::read(folder.join(format!("{name}.opus")))?;
            let (samples, header) = ruopus::decode_ogg_opus(&bytes)?;
            assert_eq!(header.channel_count, 1);
            assert!(samples.len() > 4_800);
            assert!(samples.iter().any(|sample| sample.abs() > 0.01));
        }
        Ok(())
    }
}
