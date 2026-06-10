//! Colour token types and extraction from raw strings.

use std::collections::HashMap;
use std::sync::LazyLock;

/// A parsed colour value from a Markdown document.
#[derive(Debug, Clone, PartialEq)]
pub enum ColorValue {
    Hex(u8, u8, u8),
    Rgb(u8, u8, u8),
    Named(String),
}

impl Default for ColorValue {
    fn default() -> Self {
        ColorValue::Hex(128, 128, 128)
    }
}

impl ColorValue {
    /// Returns the (r, g, b) components, resolving named colours.
    pub fn to_rgb(&self) -> (u8, u8, u8) {
        match self {
            ColorValue::Hex(r, g, b) | ColorValue::Rgb(r, g, b) => (*r, *g, *b),
            ColorValue::Named(name) => named_to_rgb(name),
        }
    }

    /// Returns the colour as a lowercase `#rrggbb` string.
    pub fn to_hex_string(&self) -> String {
        let (r, g, b) = self.to_rgb();
        format!("#{r:02x}{g:02x}{b:02x}")
    }
}

/// Extract a `ColorValue` from a string slice.
///
/// Recognises `#rrggbb`, `#rgb`, `rgb(r, g, b)`, and the 16 CSS basic colours.
pub fn extract_color(s: &str) -> Option<ColorValue> {
    let s = s.trim();

    if let Some(hex) = s.strip_prefix('#') {
        return parse_hex(hex);
    }

    if let Some(inner) = s.strip_prefix("rgb(").and_then(|t| t.strip_suffix(')')) {
        return parse_rgb(inner);
    }

    let lower = s.to_ascii_lowercase();
    if CSS_NAMED_COLORS.contains_key(lower.as_str()) {
        return Some(ColorValue::Named(lower));
    }

    None
}

fn parse_hex(hex: &str) -> Option<ColorValue> {
    // Only accept exactly 3 or 6 valid hex characters.
    if !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    match hex.len() {
        6 => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            Some(ColorValue::Hex(r, g, b))
        }
        3 => {
            let r = u8::from_str_radix(&hex[0..1], 16).ok()? * 17;
            let g = u8::from_str_radix(&hex[1..2], 16).ok()? * 17;
            let b = u8::from_str_radix(&hex[2..3], 16).ok()? * 17;
            Some(ColorValue::Hex(r, g, b))
        }
        _ => None,
    }
}

fn parse_rgb(inner: &str) -> Option<ColorValue> {
    let parts: Vec<&str> = inner.split(',').collect();
    if parts.len() != 3 {
        return None;
    }
    let r = parts[0].trim().parse::<u8>().ok()?;
    let g = parts[1].trim().parse::<u8>().ok()?;
    let b = parts[2].trim().parse::<u8>().ok()?;
    Some(ColorValue::Rgb(r, g, b))
}

fn named_to_rgb(name: &str) -> (u8, u8, u8) {
    CSS_NAMED_COLORS.get(name).copied().unwrap_or((128, 128, 128))
}

static CSS_NAMED_COLORS: LazyLock<HashMap<&'static str, (u8, u8, u8)>> =
    LazyLock::new(|| {
        let mut m = HashMap::new();
        m.insert("black",   (0,   0,   0));
        m.insert("silver",  (192, 192, 192));
        m.insert("gray",    (128, 128, 128));
        m.insert("white",   (255, 255, 255));
        m.insert("maroon",  (128, 0,   0));
        m.insert("red",     (255, 0,   0));
        m.insert("purple",  (128, 0,   128));
        m.insert("fuchsia", (255, 0,   255));
        m.insert("green",   (0,   128, 0));
        m.insert("lime",    (0,   255, 0));
        m.insert("olive",   (128, 128, 0));
        m.insert("yellow",  (255, 255, 0));
        m.insert("navy",    (0,   0,   128));
        m.insert("blue",    (0,   0,   255));
        m.insert("teal",    (0,   128, 128));
        m.insert("aqua",    (0,   255, 255));
        m
    });
