#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    String(String),
    Number(f64),
    Bool(bool),
    Null,
}

impl Value {
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
}
