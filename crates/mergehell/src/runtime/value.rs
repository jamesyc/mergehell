#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    String(String),
    Number(f64),
    Bool(bool),
    Null,
}

impl Value {
    pub fn parse_text(text: &str) -> Self {
        let trimmed = text.trim();
        if trimmed.eq_ignore_ascii_case("true") {
            Value::Bool(true)
        } else if trimmed.eq_ignore_ascii_case("false") {
            Value::Bool(false)
        } else if trimmed.eq_ignore_ascii_case("null")
            || trimmed == "deleted by us:"
            || trimmed == "deleted by them:"
        {
            Value::Null
        } else if let Ok(number) = trimmed.parse::<f64>() {
            Value::Number(number)
        } else {
            Value::String(trimmed.to_string())
        }
    }

    pub fn is_truthy(&self) -> bool {
        match self {
            Value::String(value) => !value.is_empty() && value != "false" && value != "0",
            Value::Number(value) => *value != 0.0,
            Value::Bool(value) => *value,
            Value::Null => false,
        }
    }

    pub fn as_output_text(&self) -> String {
        match self {
            Value::String(value) => value.clone(),
            Value::Number(value) => value.to_string(),
            Value::Bool(value) => value.to_string(),
            Value::Null => "null".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_values_for_output() {
        assert_eq!(Value::String("hello".to_string()).as_output_text(), "hello");
        assert_eq!(Value::Number(42.0).as_output_text(), "42");
        assert_eq!(Value::Bool(true).as_output_text(), "true");
        assert_eq!(Value::Null.as_output_text(), "null");
    }

    #[test]
    fn parses_text_values() {
        assert_eq!(Value::parse_text("true"), Value::Bool(true));
        assert_eq!(Value::parse_text("false"), Value::Bool(false));
        assert_eq!(Value::parse_text("42"), Value::Number(42.0));
        assert_eq!(Value::parse_text("null"), Value::Null);
        assert_eq!(
            Value::parse_text("hello"),
            Value::String("hello".to_string())
        );
    }

    #[test]
    fn computes_truthiness() {
        assert!(Value::String("hello".to_string()).is_truthy());
        assert!(!Value::String(String::new()).is_truthy());
        assert!(!Value::String("false".to_string()).is_truthy());
        assert!(Value::Number(1.0).is_truthy());
        assert!(!Value::Number(0.0).is_truthy());
        assert!(Value::Bool(true).is_truthy());
        assert!(!Value::Bool(false).is_truthy());
        assert!(!Value::Null.is_truthy());
    }
}
