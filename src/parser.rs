//! Markdown document parser. Wraps pulldown-cmark and extracts swatch entries.

use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};

use crate::token::{extract_token, DesignToken};

/// A single design token entry extracted from the document.
#[derive(Debug, Clone)]
pub struct SwatchEntry {
    pub name: String,
    pub token: DesignToken,
}

/// A headed group of colour swatches.
#[derive(Debug, Clone, Default)]
pub struct Section {
    pub heading: String,
    pub swatches: Vec<SwatchEntry>,
}

/// The full parsed representation of a Markdown binder document.
#[derive(Debug, Clone, Default)]
pub struct Document {
    pub sections: Vec<Section>,
}

impl Document {
    /// Flat iterator over every swatch in the document, regardless of section.
    pub fn all_swatches(&self) -> impl Iterator<Item = &SwatchEntry> {
        self.sections.iter().flat_map(|s| s.swatches.iter())
    }
}

/// Parse a Markdown string into a structured `Document`.
///
/// Colour tokens are extracted from:
/// - Inline code spans inside list items  (`- **Red** — \`#E53935\``)
/// - Bare inline code in any block        (`\`rgb(30,30,30)\``)
pub fn parse(markdown: &str) -> Document {
    let options = Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TABLES;
    let parser = Parser::new_ext(markdown, options);

    let mut doc = Document::default();
    let mut current = Section::default();
    let mut in_heading = false;
    let mut heading_buf = String::new();
    // Accumulated plain text for the current list item (used as swatch name).
    let mut item_label = String::new();
    // A token found in the current list item, waiting to be flushed.
    let mut pending: Option<DesignToken> = None;

    for event in parser {
        match event {
            Event::Start(Tag::Heading { .. }) => {
                flush_section(&mut doc, &mut current);
                in_heading = true;
                heading_buf.clear();
            }

            Event::End(TagEnd::Heading(_)) => {
                in_heading = false;
                current.heading = heading_buf.trim().to_string();
                heading_buf.clear();
            }

            Event::Start(Tag::Item) => {
                item_label.clear();
                pending = None;
            }

            Event::End(TagEnd::Item) => {
                if let Some(token) = pending.take() {
                    let name = clean_label(&item_label);
                    let fallback = token.fallback_name();
                    current.swatches.push(SwatchEntry {
                        name: if name.is_empty() { fallback } else { name },
                        token,
                    });
                }
                item_label.clear();
            }

            Event::Code(text) => {
                if in_heading {
                    // Treat code inside a heading as heading text.
                    heading_buf.push_str(&text);
                } else if let Some(token) = extract_token(&text) {
                    // Keep the last design token found in the item.
                    pending = Some(token);
                } else {
                    item_label.push_str(&text);
                }
            }

            Event::Text(text) => {
                if in_heading {
                    heading_buf.push_str(&text);
                } else {
                    item_label.push_str(&text);
                }
            }

            Event::SoftBreak | Event::HardBreak => {
                if !in_heading {
                    item_label.push(' ');
                }
            }

            _ => {}
        }
    }

    flush_section(&mut doc, &mut current);
    doc
}

fn flush_section(doc: &mut Document, section: &mut Section) {
    if !section.heading.is_empty() || !section.swatches.is_empty() {
        doc.sections.push(std::mem::take(section));
    }
}

/// Strip Markdown emphasis syntax and trailing punctuation from a label string.
fn clean_label(s: &str) -> String {
    s.replace("**", "")
        .replace("__", "")
        .replace('*', "")
        .replace('_', "")
        .trim()
        .trim_end_matches(|c: char| c == '—' || c == '-' || c == ':' || c.is_whitespace())
        .trim()
        .to_string()
}
