use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Question {
    pub text: String,
    pub normalized: String,
    pub slug: String,
}

#[derive(Debug, thiserror::Error)]
pub enum QuestionError {
    #[error("Question must be at least 10 characters")]
    TooShort,
    #[error("Question must be at most 200 characters")]
    TooLong,
    #[error("Question must end with a question mark")]
    MissingQuestionMark,
    #[error("Question contains prohibited content (HTML/script tags)")]
    ProhibitedContent,
}

pub struct QuestionParser;

impl QuestionParser {
    /// Parse and validate question text
    pub fn parse(input: &str) -> Result<Question, QuestionError> {
        // Sanitize input
        let sanitized = Self::sanitize(input);
        
        // Validate
        Self::validate(&sanitized)?;
        
        // Generate normalized form (lowercase, trimmed)
        let normalized = sanitized.trim().to_lowercase();
        
        // Generate slug
        let slug = Self::generate_slug(&sanitized);
        
        Ok(Question {
            text: sanitized,
            normalized,
            slug,
        })
    }
    
    /// Format question for display
    pub fn format(question: &Question, max_length: Option<usize>) -> String {
        let mut text = question.text.clone();
        
        // Apply capitalization (first letter uppercase)
        if let Some(first_char) = text.chars().next() {
            text = first_char.to_uppercase().collect::<String>() + &text[first_char.len_utf8()..];
        }
        
        // Truncate if needed
        if let Some(max_len) = max_length {
            if text.len() > max_len {
                text = text.chars().take(max_len - 3).collect::<String>() + "...";
            }
        }
        
        text
    }
    
    /// Validate question structure
    fn validate(text: &str) -> Result<(), QuestionError> {
        let trimmed = text.trim();
        
        // Check length
        if trimmed.len() < 10 {
            return Err(QuestionError::TooShort);
        }
        if trimmed.len() > 200 {
            return Err(QuestionError::TooLong);
        }
        
        // Check for question mark
        if !trimmed.ends_with('?') {
            return Err(QuestionError::MissingQuestionMark);
        }
        
        // Check for prohibited content
        if text.contains("<script") || text.contains("</script>") || 
           text.contains("<html") || text.contains("</html>") {
            return Err(QuestionError::ProhibitedContent);
        }
        
        Ok(())
    }
    
    /// Sanitize input (remove HTML/script tags, preserve Unicode)
    fn sanitize(text: &str) -> String {
        // Remove HTML tags
        let re = Regex::new(r"<[^>]*>").unwrap();
        let cleaned = re.replace_all(text, "");
        
        // Trim whitespace
        cleaned.trim().to_string()
    }
    
    /// Generate URL slug (kebab-case, max 50 chars)
    fn generate_slug(text: &str) -> String {
        let mut slug = text.to_lowercase();
        
        // Remove question mark
        slug = slug.replace('?', "");
        
        // Replace spaces and special chars with hyphens
        let re = Regex::new(r"[^a-z0-9]+").unwrap();
        slug = re.replace_all(&slug, "-").to_string();
        
        // Remove leading/trailing hyphens
        slug = slug.trim_matches('-').to_string();
        
        // Truncate to 50 chars
        if slug.len() > 50 {
            slug = slug.chars().take(50).collect();
            slug = slug.trim_matches('-').to_string();
        }
        
        slug
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_valid_question() {
        let result = QuestionParser::parse("Will it rain tomorrow?");
        assert!(result.is_ok());
        let q = result.unwrap();
        assert_eq!(q.text, "Will it rain tomorrow?");
        assert_eq!(q.normalized, "will it rain tomorrow?");
        assert_eq!(q.slug, "will-it-rain-tomorrow");
    }
    
    #[test]
    fn test_parse_too_short() {
        let result = QuestionParser::parse("Short?");
        assert!(matches!(result, Err(QuestionError::TooShort)));
    }
    
    #[test]
    fn test_parse_too_long() {
        let long_text = "a".repeat(201) + "?";
        let result = QuestionParser::parse(&long_text);
        assert!(matches!(result, Err(QuestionError::TooLong)));
    }
    
    #[test]
    fn test_parse_missing_question_mark() {
        let result = QuestionParser::parse("Will it rain tomorrow");
        assert!(matches!(result, Err(QuestionError::MissingQuestionMark)));
    }
    
    #[test]
    fn test_sanitize_html() {
        let result = QuestionParser::parse("<b>Will it rain</b> tomorrow?");
        assert!(result.is_ok());
        let q = result.unwrap();
        assert_eq!(q.text, "Will it rain tomorrow?");
    }
    
    #[test]
    fn test_format_truncate() {
        let q = Question {
            text: "Will it rain tomorrow?".to_string(),
            normalized: "will it rain tomorrow?".to_string(),
            slug: "will-it-rain-tomorrow".to_string(),
        };
        let formatted = QuestionParser::format(&q, Some(15));
        assert_eq!(formatted, "Will it rain...");
    }
    
    #[test]
    fn test_slug_generation() {
        let result = QuestionParser::parse("Will the guy in blue fall off the escalator?");
        assert!(result.is_ok());
        let q = result.unwrap();
        assert_eq!(q.slug, "will-the-guy-in-blue-fall-off-the-escalator");
    }
    
    #[test]
    fn test_slug_truncation() {
        let long_question = "Will this very long question that exceeds fifty characters be truncated properly?";
        let result = QuestionParser::parse(long_question);
        assert!(result.is_ok());
        let q = result.unwrap();
        assert!(q.slug.len() <= 50);
    }
}
