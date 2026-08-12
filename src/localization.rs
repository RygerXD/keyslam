use std::collections::HashMap;

use include_dir::{Dir, include_dir};

static STRINGS: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/assets/strings");

#[derive(Debug, Clone)]
pub struct Localization {
    locale: String,
    strings: HashMap<String, String>,
}

impl Localization {
    pub fn detect() -> Self {
        Self::for_locale(sys_locale::get_locale().as_deref().unwrap_or("en-EN"))
    }

    pub fn for_locale(requested: &str) -> Self {
        let normalized = requested.replace('_', "-");
        let language = normalized
            .split('-')
            .next()
            .unwrap_or("en")
            .to_ascii_lowercase();
        let candidates = [
            normalized.clone(),
            match language.as_str() {
                "de" => "de-DE".to_owned(),
                "el" => "el-GR".to_owned(),
                "es" => "es-ES".to_owned(),
                "fr" => "fr-FR".to_owned(),
                "lv" => "lv-LV".to_owned(),
                "pt" => "pt-PT".to_owned(),
                "ru" => "ru-RU".to_owned(),
                _ => "en-EN".to_owned(),
            },
            "en-EN".to_owned(),
        ];

        for candidate in candidates {
            let file_name = format!("{candidate}.json");
            let Some(file) = STRINGS.get_file(&file_name) else {
                continue;
            };
            if let Ok(strings) = serde_json::from_slice(file.contents()) {
                return Self {
                    locale: candidate,
                    strings,
                };
            }
        }

        Self {
            locale: "en-EN".to_owned(),
            strings: HashMap::new(),
        }
    }

    pub fn color_shape_audio_keys(&self, color: &str, shape: &str) -> [String; 2] {
        let format = self
            .strings
            .get("ColorShapeFormat")
            .map(String::as_str)
            .unwrap_or("{0} {1}");
        let color_key = format!("colors/{}.opus", color.to_ascii_lowercase());
        let shape_key = format!("shapes/{}.opus", shape.to_ascii_lowercase());
        if format.find("{1}") < format.find("{0}") {
            [shape_key, color_key]
        } else {
            [color_key, shape_key]
        }
    }

    pub fn locale(&self) -> &str {
        &self.locale
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn falls_back_by_language() {
        let french = Localization::for_locale("fr-CA");
        assert_eq!(french.locale(), "fr-FR");
    }

    #[test]
    fn honors_localized_word_order() {
        let portuguese = Localization::for_locale("pt-BR");
        assert_eq!(
            portuguese.color_shape_audio_keys("Red", "Circle"),
            ["shapes/circle.opus", "colors/red.opus"]
        );
    }

    #[test]
    fn missing_format_key_uses_color_then_shape() {
        let localization = Localization {
            locale: "test".to_owned(),
            strings: HashMap::new(),
        };
        assert_eq!(
            localization.color_shape_audio_keys("Red", "Star"),
            ["colors/red.opus", "shapes/star.opus"]
        );
    }
}
