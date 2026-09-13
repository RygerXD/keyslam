#[derive(Debug, Clone)]
pub struct Localization;

impl Localization {
    pub fn english() -> Self {
        Self
    }

    pub fn color_shape_audio_keys(&self, color: &str, shape: &str) -> [String; 2] {
        let color_key = format!("colors/modifier/{}", color.to_ascii_lowercase());
        let shape_key = format!("shapes/{}", shape.to_ascii_lowercase());
        [color_key, shape_key]
    }
}
