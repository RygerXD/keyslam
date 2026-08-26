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

#[cfg(test)]
pub fn response_for(key_name: &str) -> KeyResponse {
    response_for_set(key_name, "animals")
}

pub fn response_for_set(key_name: &str, extra_key_set: &str) -> KeyResponse {
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
    use super::*;

    #[test]
    fn letters_and_top_row_digits_are_glyphs() {
        assert_eq!(response_for("A").kind, ResponseKind::Glyph('A'));
        assert_eq!(response_for("5").kind, ResponseKind::Glyph('5'));
    }

    #[test]
    fn numpad_digits_are_shapes() {
        for (key, shape) in [
            ("Decimal", ShapeKind::Circle),
            ("NumPad0", ShapeKind::Oval),
            ("NumPad1", ShapeKind::Rectangle),
            ("NumPad2", ShapeKind::Heart),
            ("NumPad3", ShapeKind::Triangle),
            ("NumPad4", ShapeKind::Square),
            ("NumPad5", ShapeKind::Pentagon),
            ("NumPad6", ShapeKind::Hexagon),
            ("NumPad7", ShapeKind::Septagon),
            ("NumPad8", ShapeKind::Octagon),
            ("NumPad9", ShapeKind::Trapezoid),
            ("Multiply", ShapeKind::Star),
            ("*", ShapeKind::Star),
            ("Add", ShapeKind::Cross),
        ] {
            assert_eq!(response_for(key).kind, ResponseKind::Shape(shape));
        }
    }

    #[test]
    fn every_standard_special_key_has_a_unique_animal() {
        assert!(packs::install_builtin_packs().is_ok());
        let mut names = OTHER_KEYS
            .iter()
            .map(|(key, _)| response_for(key).spoken_text)
            .collect::<Vec<_>>();
        names.sort_unstable();
        names.dedup();
        // Multiply is intentionally a star, matching the upstream precedence rule.
        assert_eq!(names.len(), OTHER_KEYS.len());
    }

    #[test]
    fn food_set_covers_every_extra_key_and_instruments_repeat_the_catalog() {
        assert!(packs::install_builtin_packs().is_ok());
        let mut food_names = ["Escape", "Space"]
            .into_iter()
            .chain(OTHER_KEYS.iter().map(|(key, _)| *key))
            .map(|key| response_for_set(key, "foods").spoken_text)
            .collect::<Vec<_>>();
        food_names.sort_unstable();
        food_names.dedup();
        assert_eq!(food_names.len(), 69);

        let instrument = response_for_set("Escape", "instruments");
        assert_eq!(instrument.spoken_text, "Accordion");
        assert_eq!(instrument.extra_key_set.as_deref(), Some("instruments"));
    }

    #[test]
    fn named_manifest_assignment_is_used_for_delete() {
        assert_eq!(response_for_set("Delete", "animals").spoken_text, "Toucan");
    }
}
