//! Markdown parsing and terminal rendering for DocsPane

/// Kind of rendered line
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineKind {
    Heading1,
    Heading2,
    Heading3,
    Code,
    Link,
    ListItem,
    Normal,
}

/// A single rendered line
#[derive(Debug, Clone)]
pub struct RenderLine {
    pub text: String,
    pub kind: LineKind,
    pub link_target: Option<String>,
    pub anchor_id: Option<String>,
}

/// Parsed markdown document
#[derive(Debug, Clone)]
pub struct ParsedDoc {
    pub lines: Vec<RenderLine>,
}

impl ParsedDoc {
    /// Find the line index for a given anchor ID
    pub fn find_anchor(&self, anchor_id: &str) -> Option<usize> {
        self.lines
            .iter()
            .position(|line| line.anchor_id.as_deref() == Some(anchor_id))
    }
}

/// Parse markdown content into renderable lines
pub fn parse_markdown(content: &str) -> ParsedDoc {
    let mut lines = Vec::new();
    let mut in_code_block = false;

    for line in content.lines() {
        if line.starts_with("```") {
            in_code_block = !in_code_block;
            continue;
        }

        if in_code_block {
            lines.push(RenderLine {
                text: format!("  {}", line),
                kind: LineKind::Code,
                link_target: None,
                anchor_id: None,
            });
            continue;
        }

        // Handle headings
        if let Some(text) = line.strip_prefix("### ") {
            let anchor_id = make_anchor_id(text);
            lines.push(RenderLine {
                text: text.to_string(),
                kind: LineKind::Heading3,
                link_target: None,
                anchor_id: Some(anchor_id),
            });
        } else if let Some(text) = line.strip_prefix("## ") {
            let anchor_id = make_anchor_id(text);
            lines.push(RenderLine {
                text: text.to_string(),
                kind: LineKind::Heading2,
                link_target: None,
                anchor_id: Some(anchor_id),
            });
        } else if let Some(text) = line.strip_prefix("# ") {
            let anchor_id = make_anchor_id(text);
            lines.push(RenderLine {
                text: text.to_string(),
                kind: LineKind::Heading1,
                link_target: None,
                anchor_id: Some(anchor_id),
            });
        } else if line.starts_with("- ") || line.starts_with("* ") {
            // List item
            let text = format!("\u{2022} {}", &line[2..]);
            let (display, link) = extract_link(&text);
            lines.push(RenderLine {
                text: display,
                kind: if link.is_some() {
                    LineKind::Link
                } else {
                    LineKind::ListItem
                },
                link_target: link,
                anchor_id: None,
            });
        } else if line.trim().is_empty() {
            lines.push(RenderLine {
                text: String::new(),
                kind: LineKind::Normal,
                link_target: None,
                anchor_id: None,
            });
        } else {
            // Normal paragraph text - check for inline links
            let (display, link) = extract_link(line);
            lines.push(RenderLine {
                text: display,
                kind: if link.is_some() {
                    LineKind::Link
                } else {
                    LineKind::Normal
                },
                link_target: link,
                anchor_id: None,
            });
        }
    }

    ParsedDoc { lines }
}

/// Convert heading text to anchor ID
fn make_anchor_id(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .replace("--", "-")
        .trim_matches('-')
        .to_string()
}

/// Extract a markdown link from text, returning (display_text, link_target)
fn extract_link(text: &str) -> (String, Option<String>) {
    // Look for [text](target) pattern
    if let Some(start) = text.find('[') {
        if let Some(mid) = text.find("](") {
            if let Some(end) = text[mid..].find(')') {
                let link_text = &text[start + 1..mid];
                let target = &text[mid + 2..mid + end];

                // Reconstruct the display text without the link syntax
                let before = &text[..start];
                let after = &text[mid + end + 1..];
                let display = format!("{}{}{}", before, link_text, after);

                return (display, Some(target.to_string()));
            }
        }
    }

    (text.to_string(), None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_heading() {
        let doc = parse_markdown("# Hello World");
        assert_eq!(doc.lines.len(), 1);
        assert_eq!(doc.lines[0].kind, LineKind::Heading1);
        assert_eq!(doc.lines[0].text, "Hello World");
        assert_eq!(doc.lines[0].anchor_id, Some("hello-world".to_string()));
    }

    #[test]
    fn test_parse_link() {
        let (display, link) = extract_link("Check out [FM Synthesis](sources/fm.md)");
        assert_eq!(display, "Check out FM Synthesis");
        assert_eq!(link, Some("sources/fm.md".to_string()));
    }

    #[test]
    fn test_find_anchor() {
        let doc = parse_markdown("# First\n\n## Second Section\n\nText here");
        assert_eq!(doc.find_anchor("first"), Some(0));
        assert_eq!(doc.find_anchor("second-section"), Some(2));
        assert_eq!(doc.find_anchor("nonexistent"), None);
    }
}
