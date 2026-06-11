//! Colour token types and extraction from raw strings.

use std::collections::HashMap;
use std::sync::LazyLock;

// ── DesignToken ───────────────────────────────────────────────────────────────

/// A single design token extracted from an inline-code span in a Markdown list item.
///
/// Recognised inline-code syntax:
/// - `#rrggbb`, `rgb()`, `hsl()`, named colours → `Color`
/// - `font: Family Bold 16px/1.5`               → `Font`
/// - `radius: 6px`                              → `Radius`
/// - `shadow: 0 2px 8px rgba(0,0,0,0.15)`       → `Shadow`
/// - `8px`, `0.5rem`, `2em` (bare size value)   → `Space`
#[derive(Debug, Clone, PartialEq)]
pub enum DesignToken {
    Color(ColorValue),
    Font {
        family: String,
        size_px: f64,
        weight: u16,
        line_height: Option<f64>,
        /// The raw text after `font:`, preserved for display in the card.
        display: String,
    },
    Space {
        value_px: f64,
        /// Original string, e.g. `"8px"` or `"0.5rem"`.
        display: String,
    },
    Radius {
        value_px: f64,
        display: String,
    },
    Shadow(String),
}

impl DesignToken {
    /// A short string used as the card name when the list item has no label.
    pub fn fallback_name(&self) -> String {
        match self {
            DesignToken::Color(c) => c.to_hex_string(),
            DesignToken::Font { display, .. } => format!("font: {display}"),
            DesignToken::Space { display, .. } => display.clone(),
            DesignToken::Radius { display, .. } => format!("radius: {display}"),
            DesignToken::Shadow(css) => format!("shadow: {css}"),
        }
    }

    /// Return the inner `ColorValue` if this is a `Color` token.
    pub fn as_color(&self) -> Option<&ColorValue> {
        match self {
            DesignToken::Color(c) => Some(c),
            _ => None,
        }
    }
}

/// Extract a `DesignToken` from a raw inline-code string.
///
/// Priority order: colour → `font:` → `radius:` → `shadow:` → bare size.
pub fn extract_token(s: &str) -> Option<DesignToken> {
    let s = s.trim();

    if let Some(color) = extract_color(s) {
        return Some(DesignToken::Color(color));
    }

    if let Some(rest) = s.strip_prefix("font:") {
        return parse_font_token(rest.trim());
    }

    if let Some(rest) = s.strip_prefix("radius:") {
        let rest = rest.trim();
        let value_px = parse_size_to_px(rest)?;
        return Some(DesignToken::Radius {
            value_px,
            display: rest.to_string(),
        });
    }

    if let Some(rest) = s.strip_prefix("shadow:") {
        let rest = rest.trim();
        if !rest.is_empty() {
            return Some(DesignToken::Shadow(rest.to_string()));
        }
        return None;
    }

    if let Some(value_px) = parse_size_to_px(s) {
        return Some(DesignToken::Space {
            value_px,
            display: s.to_string(),
        });
    }

    None
}

/// Parse `font: [family] [weight] <size>[/line-height]` into a `Font` token.
fn parse_font_token(s: &str) -> Option<DesignToken> {
    let display = s.to_string();

    // Split off an optional /line-height suffix (e.g. `Inter 16px/1.5`).
    let (main_str, line_height) = match s.rfind('/') {
        Some(pos) => {
            let lh_str = s[pos + 1..].trim();
            // Accept unitless multiplier (`1.5`) or a px/rem length.
            let lh = lh_str
                .parse::<f64>()
                .ok()
                .or_else(|| parse_size_to_px(lh_str).map(|px| px / 16.0));
            (s[..pos].trim(), lh)
        }
        None => (s, None),
    };

    // Handle a quoted family name, e.g. `"JetBrains Mono"`.
    let (quoted_family, remainder) = if main_str.starts_with('"') || main_str.starts_with('\'') {
        let q = main_str.chars().next()?;
        let close = main_str[1..].find(q)? + 1;
        (
            Some(main_str[1..close].to_string()),
            main_str[close + 1..].trim(),
        )
    } else {
        (None, main_str)
    };

    let tokens: Vec<&str> = remainder.split_whitespace().collect();
    if tokens.is_empty() && quoted_family.is_none() {
        return None;
    }

    // Find the size (rightmost token that looks like `16px`, `1rem`, `1.5em`).
    let size_pos = tokens.iter().rposition(|t| parse_size_to_px(t).is_some())?;
    let size_px = parse_size_to_px(tokens[size_pos])?;

    // Weight: the token immediately before size, if it's a weight keyword/number.
    let (weight, family_end) = if size_pos > 0 {
        if let Some(w) = weight_from_str(tokens[size_pos - 1]) {
            (w, size_pos - 1)
        } else {
            (400u16, size_pos)
        }
    } else {
        (400u16, 0)
    };

    // Family: quoted name, or unquoted tokens[0..family_end] joined.
    let family = match quoted_family {
        Some(f) => f,
        None if family_end > 0 => tokens[..family_end].join(" "),
        _ => "sans-serif".to_string(),
    };

    Some(DesignToken::Font {
        family,
        size_px,
        weight,
        line_height,
        display,
    })
}

/// Convert a CSS font-weight keyword or numeric string to a CSS weight integer.
fn weight_from_str(s: &str) -> Option<u16> {
    match s.to_ascii_lowercase().as_str() {
        "thin" => Some(100),
        "extralight" | "extra-light" | "ultralight" => Some(200),
        "light" => Some(300),
        "regular" | "normal" => Some(400),
        "medium" => Some(500),
        "semibold" | "semi-bold" | "demibold" => Some(600),
        "bold" => Some(700),
        "extrabold" | "extra-bold" | "ultrabold" => Some(800),
        "black" | "heavy" => Some(900),
        _ => s.parse::<u16>().ok().filter(|&w| (100..=900).contains(&w)),
    }
}

/// Convert a CSS size string (`16px`, `1rem`, `1.5em`) to pixels.
/// `rem`/`em` assume a 16px root font size.
fn parse_size_to_px(s: &str) -> Option<f64> {
    if let Some(v) = s.strip_suffix("px") {
        v.parse::<f64>().ok().filter(|&n| n >= 0.0)
    } else if let Some(v) = s.strip_suffix("rem") {
        v.parse::<f64>().ok().map(|n| n * 16.0)
    } else if let Some(v) = s.strip_suffix("em") {
        v.parse::<f64>().ok().map(|n| n * 16.0)
    } else {
        None
    }
}

// ── ColorValue ────────────────────────────────────────────────────────────────

/// A parsed colour value from a Markdown document.
///
/// All numeric formats (`#hex`, `rgb()`, `rgba()`, `hsl()`, `hsla()`) collapse
/// into the single `Rgba` variant — the original syntax carried no meaning once
/// resolved to channels. `Named` is kept distinct so we can preserve the
/// author's spelling (e.g. `red`) for future write-back and resolve it lazily.
#[derive(Debug, Clone, PartialEq)]
pub enum ColorValue {
    /// Resolved red, green, blue, alpha — each 0..=255.
    Rgba(u8, u8, u8, u8),
    /// A CSS named colour, resolved on demand. Always fully opaque.
    Named(String),
}

impl Default for ColorValue {
    fn default() -> Self {
        ColorValue::Rgba(128, 128, 128, 255)
    }
}

impl ColorValue {
    /// Returns the (r, g, b, a) components, resolving named colours (alpha 255).
    pub fn to_rgba(&self) -> (u8, u8, u8, u8) {
        match self {
            ColorValue::Rgba(r, g, b, a) => (*r, *g, *b, *a),
            ColorValue::Named(name) => {
                let (r, g, b) = named_to_rgb(name);
                (r, g, b, 255)
            }
        }
    }

    /// Returns the opaque (r, g, b) components, discarding any alpha.
    pub fn to_rgb(&self) -> (u8, u8, u8) {
        let (r, g, b, _) = self.to_rgba();
        (r, g, b)
    }

    /// The alpha channel, 0..=255 (255 = fully opaque).
    pub fn alpha(&self) -> u8 {
        self.to_rgba().3
    }

    /// Returns the colour as a lowercase hex string — `#rrggbb` when opaque,
    /// `#rrggbbaa` when it carries transparency.
    pub fn to_hex_string(&self) -> String {
        let (r, g, b, a) = self.to_rgba();
        if a == 255 {
            format!("#{r:02x}{g:02x}{b:02x}")
        } else {
            format!("#{r:02x}{g:02x}{b:02x}{a:02x}")
        }
    }
}

/// Extract a `ColorValue` from a string slice.
///
/// Recognises:
/// - `#rgb`, `#rgba`, `#rrggbb`, `#rrggbbaa`
/// - `rgb(r, g, b)` and `rgba(r, g, b, a)` (alpha as 0..1 float or 0..255)
/// - `hsl(h, s%, l%)` and `hsla(h, s%, l%, a)`
/// - any of the 148 CSS named colours
pub fn extract_color(s: &str) -> Option<ColorValue> {
    let s = s.trim();

    if let Some(hex) = s.strip_prefix('#') {
        return parse_hex(hex);
    }

    // Accept both `rgba(` and `rgb(` — strip whichever prefix matches.
    if let Some(inner) = strip_fn(s, "rgba").or_else(|| strip_fn(s, "rgb")) {
        return parse_rgb(inner);
    }

    if let Some(inner) = strip_fn(s, "hsla").or_else(|| strip_fn(s, "hsl")) {
        return parse_hsl(inner);
    }

    let lower = s.to_ascii_lowercase();
    if lower == "transparent" {
        return Some(ColorValue::Rgba(0, 0, 0, 0));
    }
    if CSS_NAMED_COLORS.contains_key(lower.as_str()) {
        return Some(ColorValue::Named(lower));
    }

    None
}

/// Strip a `name(...)` wrapper, returning the inner argument list.
fn strip_fn<'a>(s: &'a str, name: &str) -> Option<&'a str> {
    s.strip_prefix(name)
        .and_then(|t| t.trim_start().strip_prefix('('))
        .and_then(|t| t.strip_suffix(')'))
}

fn parse_hex(hex: &str) -> Option<ColorValue> {
    if !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    // Expand shorthand (`#rgb` / `#rgba`) by doubling each nibble so every form
    // becomes an even run of hex pairs we can read uniformly.
    let expanded = match hex.len() {
        3 | 4 => hex.chars().flat_map(|c| [c, c]).collect::<String>(),
        6 | 8 => hex.to_string(),
        _ => return None,
    };
    let byte = |i: usize| u8::from_str_radix(&expanded[i * 2..i * 2 + 2], 16).ok();
    let a = if expanded.len() == 8 { byte(3)? } else { 255 };
    Some(ColorValue::Rgba(byte(0)?, byte(1)?, byte(2)?, a))
}

fn parse_rgb(inner: &str) -> Option<ColorValue> {
    let parts: Vec<&str> = inner.split(',').collect();
    if parts.len() != 3 && parts.len() != 4 {
        return None;
    }
    let r = parts[0].trim().parse::<u8>().ok()?;
    let g = parts[1].trim().parse::<u8>().ok()?;
    let b = parts[2].trim().parse::<u8>().ok()?;
    let a = match parts.get(3) {
        Some(p) => parse_alpha(p.trim())?,
        None => 255,
    };
    Some(ColorValue::Rgba(r, g, b, a))
}

fn parse_hsl(inner: &str) -> Option<ColorValue> {
    let parts: Vec<&str> = inner.split(',').collect();
    if parts.len() != 3 && parts.len() != 4 {
        return None;
    }
    let h = parts[0]
        .trim()
        .trim_end_matches("deg")
        .parse::<f64>()
        .ok()?;
    let s = parts[1].trim().trim_end_matches('%').parse::<f64>().ok()? / 100.0;
    let l = parts[2].trim().trim_end_matches('%').parse::<f64>().ok()? / 100.0;
    let a = match parts.get(3) {
        Some(p) => parse_alpha(p.trim())?,
        None => 255,
    };
    let (r, g, b) = hsl_to_rgb(h, s, l);
    Some(ColorValue::Rgba(r, g, b, a))
}

/// Parse an alpha argument: a `0..1` float (CSS), a `0..1` value with `%`, or a
/// raw `0..255` integer. Clamped to a `u8`.
fn parse_alpha(s: &str) -> Option<u8> {
    if let Some(pct) = s.strip_suffix('%') {
        let v = pct.trim().parse::<f64>().ok()?;
        return Some(((v / 100.0).clamp(0.0, 1.0) * 255.0).round() as u8);
    }
    let v = s.parse::<f64>().ok()?;
    if v <= 1.0 {
        Some((v.clamp(0.0, 1.0) * 255.0).round() as u8)
    } else {
        Some(v.clamp(0.0, 255.0).round() as u8)
    }
}

/// Convert HSL (hue in degrees, saturation/lightness in 0..1) to 8-bit RGB.
fn hsl_to_rgb(h: f64, s: f64, l: f64) -> (u8, u8, u8) {
    let h = h.rem_euclid(360.0) / 360.0;
    let s = s.clamp(0.0, 1.0);
    let l = l.clamp(0.0, 1.0);

    if s == 0.0 {
        let v = (l * 255.0).round() as u8;
        return (v, v, v);
    }

    let q = if l < 0.5 {
        l * (1.0 + s)
    } else {
        l + s - l * s
    };
    let p = 2.0 * l - q;
    let to_u8 = |t: f64| (hue_to_channel(p, q, t) * 255.0).round() as u8;
    (to_u8(h + 1.0 / 3.0), to_u8(h), to_u8(h - 1.0 / 3.0))
}

fn hue_to_channel(p: f64, q: f64, t: f64) -> f64 {
    let t = t.rem_euclid(1.0);
    if t < 1.0 / 6.0 {
        p + (q - p) * 6.0 * t
    } else if t < 1.0 / 2.0 {
        q
    } else if t < 2.0 / 3.0 {
        p + (q - p) * (2.0 / 3.0 - t) * 6.0
    } else {
        p
    }
}

fn named_to_rgb(name: &str) -> (u8, u8, u8) {
    CSS_NAMED_COLORS
        .get(name)
        .copied()
        .unwrap_or((128, 128, 128))
}

// ── WCAG contrast ─────────────────────────────────────────────────────────────

/// Relative luminance of an sRGB colour per the WCAG 2.x definition.
fn relative_luminance(rgb: (u8, u8, u8)) -> f64 {
    // Linearise each channel: small values are scaled linearly, the rest follow
    // a gamma curve. These constants are taken verbatim from the WCAG spec.
    let lin = |c: u8| {
        let c = c as f64 / 255.0;
        if c <= 0.03928 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    let (r, g, b) = rgb;
    0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b)
}

/// WCAG contrast ratio between two opaque colours, always >= 1.0 (max 21.0).
///
/// 4.5 is the AA threshold for normal text, 7.0 the AAA threshold.
pub fn contrast_ratio(a: (u8, u8, u8), b: (u8, u8, u8)) -> f64 {
    let la = relative_luminance(a);
    let lb = relative_luminance(b);
    let (hi, lo) = if la >= lb { (la, lb) } else { (lb, la) };
    (hi + 0.05) / (lo + 0.05)
}

/// The WCAG conformance level a contrast ratio achieves for normal-size text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WcagLevel {
    Aaa,
    Aa,
    Fail,
}

impl WcagLevel {
    pub fn for_ratio(ratio: f64) -> Self {
        if ratio >= 7.0 {
            WcagLevel::Aaa
        } else if ratio >= 4.5 {
            WcagLevel::Aa
        } else {
            WcagLevel::Fail
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            WcagLevel::Aaa => "AAA",
            WcagLevel::Aa => "AA",
            WcagLevel::Fail => "Fail",
        }
    }
}

/// The 148 CSS named colours (CSS Color Module Level 4), including the
/// British-spelling `grey` aliases and `rebeccapurple`.
static CSS_NAMED_COLORS: LazyLock<HashMap<&'static str, (u8, u8, u8)>> = LazyLock::new(|| {
    [
        ("aliceblue", (240, 248, 255)),
        ("antiquewhite", (250, 235, 215)),
        ("aqua", (0, 255, 255)),
        ("aquamarine", (127, 255, 212)),
        ("azure", (240, 255, 255)),
        ("beige", (245, 245, 220)),
        ("bisque", (255, 228, 196)),
        ("black", (0, 0, 0)),
        ("blanchedalmond", (255, 235, 205)),
        ("blue", (0, 0, 255)),
        ("blueviolet", (138, 43, 226)),
        ("brown", (165, 42, 42)),
        ("burlywood", (222, 184, 135)),
        ("cadetblue", (95, 158, 160)),
        ("chartreuse", (127, 255, 0)),
        ("chocolate", (210, 105, 30)),
        ("coral", (255, 127, 80)),
        ("cornflowerblue", (100, 149, 237)),
        ("cornsilk", (255, 248, 220)),
        ("crimson", (220, 20, 60)),
        ("cyan", (0, 255, 255)),
        ("darkblue", (0, 0, 139)),
        ("darkcyan", (0, 139, 139)),
        ("darkgoldenrod", (184, 134, 11)),
        ("darkgray", (169, 169, 169)),
        ("darkgreen", (0, 100, 0)),
        ("darkgrey", (169, 169, 169)),
        ("darkkhaki", (189, 183, 107)),
        ("darkmagenta", (139, 0, 139)),
        ("darkolivegreen", (85, 107, 47)),
        ("darkorange", (255, 140, 0)),
        ("darkorchid", (153, 50, 204)),
        ("darkred", (139, 0, 0)),
        ("darksalmon", (233, 150, 122)),
        ("darkseagreen", (143, 188, 143)),
        ("darkslateblue", (72, 61, 139)),
        ("darkslategray", (47, 79, 79)),
        ("darkslategrey", (47, 79, 79)),
        ("darkturquoise", (0, 206, 209)),
        ("darkviolet", (148, 0, 211)),
        ("deeppink", (255, 20, 147)),
        ("deepskyblue", (0, 191, 255)),
        ("dimgray", (105, 105, 105)),
        ("dimgrey", (105, 105, 105)),
        ("dodgerblue", (30, 144, 255)),
        ("firebrick", (178, 34, 34)),
        ("floralwhite", (255, 250, 240)),
        ("forestgreen", (34, 139, 34)),
        ("fuchsia", (255, 0, 255)),
        ("gainsboro", (220, 220, 220)),
        ("ghostwhite", (248, 248, 255)),
        ("gold", (255, 215, 0)),
        ("goldenrod", (218, 165, 32)),
        ("gray", (128, 128, 128)),
        ("green", (0, 128, 0)),
        ("greenyellow", (173, 255, 47)),
        ("grey", (128, 128, 128)),
        ("honeydew", (240, 255, 240)),
        ("hotpink", (255, 105, 180)),
        ("indianred", (205, 92, 92)),
        ("indigo", (75, 0, 130)),
        ("ivory", (255, 255, 240)),
        ("khaki", (240, 230, 140)),
        ("lavender", (230, 230, 250)),
        ("lavenderblush", (255, 240, 245)),
        ("lawngreen", (124, 252, 0)),
        ("lemonchiffon", (255, 250, 205)),
        ("lightblue", (173, 216, 230)),
        ("lightcoral", (240, 128, 128)),
        ("lightcyan", (224, 255, 255)),
        ("lightgoldenrodyellow", (250, 250, 210)),
        ("lightgray", (211, 211, 211)),
        ("lightgreen", (144, 238, 144)),
        ("lightgrey", (211, 211, 211)),
        ("lightpink", (255, 182, 193)),
        ("lightsalmon", (255, 160, 122)),
        ("lightseagreen", (32, 178, 170)),
        ("lightskyblue", (135, 206, 250)),
        ("lightslategray", (119, 136, 153)),
        ("lightslategrey", (119, 136, 153)),
        ("lightsteelblue", (176, 196, 222)),
        ("lightyellow", (255, 255, 224)),
        ("lime", (0, 255, 0)),
        ("limegreen", (50, 205, 50)),
        ("linen", (250, 240, 230)),
        ("magenta", (255, 0, 255)),
        ("maroon", (128, 0, 0)),
        ("mediumaquamarine", (102, 205, 170)),
        ("mediumblue", (0, 0, 205)),
        ("mediumorchid", (186, 85, 211)),
        ("mediumpurple", (147, 112, 219)),
        ("mediumseagreen", (60, 179, 113)),
        ("mediumslateblue", (123, 104, 238)),
        ("mediumspringgreen", (0, 250, 154)),
        ("mediumturquoise", (72, 209, 204)),
        ("mediumvioletred", (199, 21, 133)),
        ("midnightblue", (25, 25, 112)),
        ("mintcream", (245, 255, 250)),
        ("mistyrose", (255, 228, 225)),
        ("moccasin", (255, 228, 181)),
        ("navajowhite", (255, 222, 173)),
        ("navy", (0, 0, 128)),
        ("oldlace", (253, 245, 230)),
        ("olive", (128, 128, 0)),
        ("olivedrab", (107, 142, 35)),
        ("orange", (255, 165, 0)),
        ("orangered", (255, 69, 0)),
        ("orchid", (218, 112, 214)),
        ("palegoldenrod", (238, 232, 170)),
        ("palegreen", (152, 251, 152)),
        ("paleturquoise", (175, 238, 238)),
        ("palevioletred", (219, 112, 147)),
        ("papayawhip", (255, 239, 213)),
        ("peachpuff", (255, 218, 185)),
        ("peru", (205, 133, 63)),
        ("pink", (255, 192, 203)),
        ("plum", (221, 160, 221)),
        ("powderblue", (176, 224, 230)),
        ("purple", (128, 0, 128)),
        ("rebeccapurple", (102, 51, 153)),
        ("red", (255, 0, 0)),
        ("rosybrown", (188, 143, 143)),
        ("royalblue", (65, 105, 225)),
        ("saddlebrown", (139, 69, 19)),
        ("salmon", (250, 128, 114)),
        ("sandybrown", (244, 164, 96)),
        ("seagreen", (46, 139, 87)),
        ("seashell", (255, 245, 238)),
        ("sienna", (160, 82, 45)),
        ("silver", (192, 192, 192)),
        ("skyblue", (135, 206, 235)),
        ("slateblue", (106, 90, 205)),
        ("slategray", (112, 128, 144)),
        ("slategrey", (112, 128, 144)),
        ("snow", (255, 250, 250)),
        ("springgreen", (0, 255, 127)),
        ("steelblue", (70, 130, 180)),
        ("tan", (210, 180, 140)),
        ("teal", (0, 128, 128)),
        ("thistle", (216, 191, 216)),
        ("tomato", (255, 99, 71)),
        ("turquoise", (64, 224, 208)),
        ("violet", (238, 130, 238)),
        ("wheat", (245, 222, 179)),
        ("white", (255, 255, 255)),
        ("whitesmoke", (245, 245, 245)),
        ("yellow", (255, 255, 0)),
        ("yellowgreen", (154, 205, 50)),
    ]
    .into_iter()
    .collect()
});
