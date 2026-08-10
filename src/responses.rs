#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShapeKind {
    Star,
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
    Emoji(&'static str),
    Shape(ShapeKind),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyResponse {
    pub kind: ResponseKind,
    pub spoken_text: &'static str,
}

impl KeyResponse {
    const fn animal(emoji: &'static str, name: &'static str) -> Self {
        Self {
            kind: ResponseKind::Emoji(emoji),
            spoken_text: name,
        }
    }

    const fn shape(shape: ShapeKind) -> Self {
        Self {
            kind: ResponseKind::Shape(shape),
            spoken_text: shape.name(),
        }
    }
}

const OTHER_KEYS: [&str; 67] = [
    "F1",
    "F2",
    "F3",
    "F4",
    "F5",
    "F6",
    "F7",
    "F8",
    "F9",
    "F10",
    "F11",
    "F12",
    "Snapshot",
    "Scroll",
    "Pause",
    "Oem3",
    "OemMinus",
    "OemPlus",
    "Back",
    "Tab",
    "Oem4",
    "Oem6",
    "Oem5",
    "Capital",
    "Oem1",
    "Oem7",
    "Return",
    "LeftShift",
    "OemComma",
    "OemPeriod",
    "Oem2",
    "RightShift",
    "LeftCtrl",
    "LWin",
    "LeftAlt",
    "RightAlt",
    "RWin",
    "Apps",
    "RightCtrl",
    "Insert",
    "Home",
    "Prior",
    "Delete",
    "End",
    "Next",
    "Left",
    "Up",
    "Down",
    "Right",
    "NumLock",
    "Divide",
    "Multiply",
    "Subtract",
    "Add",
    "Decimal",
    "F13",
    "F14",
    "F15",
    "F16",
    "F17",
    "F18",
    "F19",
    "F20",
    "F21",
    "F22",
    "F23",
    "F24",
];

const OTHER_ANIMALS: [(&str, &str); 67] = [
    ("🐝", "Bee"),
    ("🦍", "Gorilla"),
    ("🦁", "Lion"),
    ("🐘", "Elephant"),
    ("🐵", "Monkey"),
    ("🐕", "Dog"),
    ("🐈", "Cat"),
    ("🐰", "Rabbit"),
    ("🐎", "Horse"),
    ("🐄", "Cow"),
    ("🐖", "Pig"),
    ("🐑", "Sheep"),
    ("🐐", "Goat"),
    ("🐔", "Chicken"),
    ("🦆", "Duck"),
    ("🐸", "Frog"),
    ("🐢", "Turtle"),
    ("🐟", "Fish"),
    ("🐋", "Whale"),
    ("🐬", "Dolphin"),
    ("🦈", "Shark"),
    ("🐙", "Octopus"),
    ("🦀", "Crab"),
    ("🦋", "Butterfly"),
    ("🐞", "Ladybug"),
    ("🐌", "Snail"),
    ("🐜", "Ant"),
    ("🕷️", "Spider"),
    ("🦉", "Owl"),
    ("🐧", "Penguin"),
    ("🦜", "Parrot"),
    ("🦅", "Eagle"),
    ("🦒", "Giraffe"),
    ("🦓", "Zebra"),
    ("🐼", "Panda"),
    ("🐨", "Koala"),
    ("🦘", "Kangaroo"),
    ("🐪", "Camel"),
    ("🦛", "Hippopotamus"),
    ("🦏", "Rhinoceros"),
    ("🦊", "Fox"),
    ("🦌", "Deer"),
    ("🐊", "Crocodile"),
    ("🐍", "Snake"),
    ("🦎", "Lizard"),
    ("🦭", "Seal"),
    ("🦇", "Bat"),
    ("🐁", "Mouse"),
    ("🫏", "Donkey"),
    ("🫎", "Moose"),
    ("🦝", "Raccoon"),
    ("🐿️", "Squirrel"),
    ("🦔", "Hedgehog"),
    ("🦦", "Otter"),
    ("🦃", "Turkey"),
    ("🐥", "Chick"),
    ("🦩", "Flamingo"),
    ("🦚", "Peacock"),
    ("🪿", "Goose"),
    ("🐛", "Caterpillar"),
    ("🦧", "Orangutan"),
    ("🦬", "Bison"),
    ("🦙", "Llama"),
    ("🦫", "Beaver"),
    ("🦥", "Sloth"),
    ("🦨", "Skunk"),
    ("🦡", "Badger"),
];

pub fn response_for(key_name: &str) -> KeyResponse {
    match key_name {
        "NumPad1" => return KeyResponse::shape(ShapeKind::Oval),
        "NumPad2" => return KeyResponse::shape(ShapeKind::Rectangle),
        "NumPad3" => return KeyResponse::shape(ShapeKind::Triangle),
        "NumPad4" => return KeyResponse::shape(ShapeKind::Square),
        "NumPad5" => return KeyResponse::shape(ShapeKind::Pentagon),
        "NumPad6" => return KeyResponse::shape(ShapeKind::Hexagon),
        "NumPad7" => return KeyResponse::shape(ShapeKind::Septagon),
        "NumPad8" => return KeyResponse::shape(ShapeKind::Octagon),
        "NumPad9" => return KeyResponse::shape(ShapeKind::Trapezoid),
        "NumPad0" => return KeyResponse::shape(ShapeKind::Circle),
        "Multiply" => return KeyResponse::shape(ShapeKind::Star),
        "Escape" => return KeyResponse::animal("🐻", "Bear"),
        "Space" => return KeyResponse::animal("🐯", "Tiger"),
        _ => {}
    }

    let normalized = normalize_key_name(key_name);
    if let Some(index) = OTHER_KEYS
        .iter()
        .position(|candidate| *candidate == normalized)
    {
        let (emoji, name) = OTHER_ANIMALS[index];
        return KeyResponse::animal(emoji, name);
    }

    let bytes = normalized.as_bytes();
    if bytes.len() == 1 && bytes[0].is_ascii_alphabetic() {
        let glyph = bytes[0].to_ascii_uppercase() as char;
        return KeyResponse {
            kind: ResponseKind::Glyph(glyph),
            spoken_text: "",
        };
    }
    if bytes.len() == 2 && bytes[0] == b'D' && bytes[1].is_ascii_digit() {
        return KeyResponse {
            kind: ResponseKind::Glyph(bytes[1] as char),
            spoken_text: "",
        };
    }

    let index = stable_hash(normalized) as usize % OTHER_ANIMALS.len();
    let (emoji, name) = OTHER_ANIMALS[index];
    KeyResponse::animal(emoji, name)
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
        assert_eq!(
            response_for("NumPad5").kind,
            ResponseKind::Shape(ShapeKind::Pentagon)
        );
    }

    #[test]
    fn every_standard_special_key_has_a_unique_animal() {
        let mut names = OTHER_KEYS
            .iter()
            .map(|key| response_for(key).spoken_text)
            .collect::<Vec<_>>();
        names.sort_unstable();
        names.dedup();
        // Multiply is intentionally a star, matching the upstream precedence rule.
        assert_eq!(names.len(), OTHER_KEYS.len());
    }
}
