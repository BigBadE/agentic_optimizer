use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

/// Configuration for text width calculation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmojiMode {
    /// Strip all emojis and replace with safe fallback
    Strict,
    /// Allow emojis with best-effort width calculation
    Permissive,
    /// Replace emojis with text representations (e.g., :smile:)
    TextFallback,
}

impl Default for EmojiMode {
    fn default() -> Self {
        Self::Permissive
    }
}

/// Calculate the display width of a string, handling emojis and grapheme clusters correctly
pub fn calculate_width(text: &str, mode: EmojiMode) -> usize {
    text.graphemes(true)
        .map(|grapheme| grapheme_width(grapheme, mode))
        .sum()
}

/// Calculate the width of a single grapheme cluster
fn grapheme_width(grapheme: &str, mode: EmojiMode) -> usize {
    // Check if this is an emoji or contains emoji modifiers
    if is_emoji_grapheme(grapheme) {
        match mode {
            EmojiMode::Strict => 1, // Replace with single-width fallback
            EmojiMode::TextFallback => 7, // Approximate width of :emoji:
            EmojiMode::Permissive => {
                // Best-effort width calculation for emojis
                if contains_emoji_modifier(grapheme) || is_zwj_sequence(grapheme) {
                    2 // Most emojis are double-width
                } else {
                    UnicodeWidthStr::width(grapheme).max(2)
                }
            }
        }
    } else {
        // Regular text - use standard width calculation
        UnicodeWidthStr::width(grapheme)
    }
}

/// Check if a grapheme is an emoji
fn is_emoji_grapheme(grapheme: &str) -> bool {
    grapheme.chars().any(|c| is_emoji_char(c))
}

/// Check if a character is an emoji
fn is_emoji_char(c: char) -> bool {
    matches!(c,
        // Emoji ranges
        '\u{1F600}'..='\u{1F64F}' | // Emoticons
        '\u{1F300}'..='\u{1F5FF}' | // Misc Symbols and Pictographs
        '\u{1F680}'..='\u{1F6FF}' | // Transport and Map
        '\u{1F1E0}'..='\u{1F1FF}' | // Regional Indicators (flags)
        '\u{2600}'..='\u{26FF}'   | // Misc symbols
        '\u{2700}'..='\u{27BF}'   | // Dingbats
        '\u{1F900}'..='\u{1F9FF}' | // Supplemental Symbols and Pictographs
        '\u{1FA00}'..='\u{1FA6F}' | // Chess Symbols
        '\u{1FA70}'..='\u{1FAFF}' | // Symbols and Pictographs Extended-A
        '\u{FE00}'..='\u{FE0F}'   | // Variation Selectors
        '\u{200D}'                  // Zero Width Joiner
    )
}

/// Check if grapheme contains emoji modifiers or variation selectors
fn contains_emoji_modifier(grapheme: &str) -> bool {
    grapheme.chars().any(|c| matches!(c,
        '\u{FE00}'..='\u{FE0F}' | // Variation Selectors
        '\u{1F3FB}'..='\u{1F3FF}'   // Skin tone modifiers
    ))
}

/// Check if grapheme is a ZWJ (Zero Width Joiner) sequence
fn is_zwj_sequence(grapheme: &str) -> bool {
    grapheme.contains('\u{200D}')
}

/// Truncate text to fit within a maximum width, respecting grapheme boundaries
pub fn truncate_to_width(text: &str, max_width: usize, mode: EmojiMode) -> String {
    let mut result = String::new();
    let mut current_width = 0;
    
    for grapheme in text.graphemes(true) {
        let grapheme_w = grapheme_width(grapheme, mode);
        
        if current_width + grapheme_w > max_width {
            break;
        }
        
        result.push_str(grapheme);
        current_width += grapheme_w;
    }
    
    result
}

/// Strip emojis from text, replacing with safe fallback
pub fn strip_emojis(text: &str, fallback: &str) -> String {
    text.graphemes(true)
        .map(|grapheme| {
            if is_emoji_grapheme(grapheme) {
                fallback
            } else {
                grapheme
            }
        })
        .collect()
}

/// Replace emojis with text representations
#[allow(dead_code)]
pub fn replace_emojis_with_text(text: &str) -> String {
    text.graphemes(true)
        .map(|grapheme| {
            if is_emoji_grapheme(grapheme) {
                emoji_to_text(grapheme)
            } else {
                grapheme.to_string()
            }
        })
        .collect()
}

/// Convert emoji to text representation (simplified)
#[allow(dead_code)]
fn emoji_to_text(emoji: &str) -> String {
    // Map common emojis to text
    match emoji {
        "💭" => ":thought:".to_string(),
        "🔧" => ":tool:".to_string(),
        "📝" => ":memo:".to_string(),
        "✓" | "✅" => ":check:".to_string(),
        "✗" | "❌" => ":cross:".to_string(),
        "⚠" | "⚠️" => ":warning:".to_string(),
        "ℹ" | "ℹ️" => ":info:".to_string(),
        _ => ":emoji:".to_string(),
    }
}

/// Wrap text to fit within a maximum width, respecting grapheme boundaries
pub fn wrap_text(text: &str, max_width: usize, mode: EmojiMode) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current_line = String::new();
    let mut current_width = 0;
    
    for grapheme in text.graphemes(true) {
        let grapheme_w = grapheme_width(grapheme, mode);
        
        // Handle newlines
        if grapheme == "\n" {
            lines.push(current_line);
            current_line = String::new();
            current_width = 0;
            continue;
        }
        
        // Check if adding this grapheme would exceed max width
        if current_width + grapheme_w > max_width && !current_line.is_empty() {
            lines.push(current_line);
            current_line = String::new();
            current_width = 0;
        }
        
        current_line.push_str(grapheme);
        current_width += grapheme_w;
    }
    
    if !current_line.is_empty() {
        lines.push(current_line);
    }
    
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ascii_width() {
        assert_eq!(calculate_width("hello", EmojiMode::Permissive), 5);
        assert_eq!(calculate_width("test", EmojiMode::Strict), 4);
    }

    #[test]
    fn test_emoji_width() {
        // Single emoji
        assert_eq!(calculate_width("💭", EmojiMode::Permissive), 2);
        assert_eq!(calculate_width("🔧", EmojiMode::Permissive), 2);
        
        // Emoji in strict mode
        assert_eq!(calculate_width("💭", EmojiMode::Strict), 1);
    }

    #[test]
    fn test_mixed_content() {
        let text = "💭 Thinking";
        assert!(calculate_width(text, EmojiMode::Permissive) >= 10);
    }

    #[test]
    fn test_truncate() {
        let text = "Hello World";
        assert_eq!(truncate_to_width(text, 5, EmojiMode::Permissive), "Hello");
        
        let emoji_text = "💭 Test";
        let truncated = truncate_to_width(emoji_text, 5, EmojiMode::Permissive);
        assert!(calculate_width(&truncated, EmojiMode::Permissive) <= 5);
    }

    #[test]
    fn test_strip_emojis() {
        assert_eq!(strip_emojis("💭 Hello", "?"), "? Hello");
        assert_eq!(strip_emojis("Test 🔧 Tool", "*"), "Test * Tool");
    }

    #[test]
    fn test_wrap_text() {
        let text = "Hello World Test";
        let wrapped = wrap_text(text, 10, EmojiMode::Permissive);
        assert!(wrapped.len() >= 2);
        
        for line in &wrapped {
            assert!(calculate_width(line, EmojiMode::Permissive) <= 10);
        }
    }

    #[test]
    fn test_emoji_detection() {
        assert!(is_emoji_grapheme("💭"));
        assert!(is_emoji_grapheme("🔧"));
        assert!(!is_emoji_grapheme("a"));
        assert!(!is_emoji_grapheme("A"));
    }
}
