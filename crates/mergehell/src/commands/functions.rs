pub fn is_available() -> bool {
    true
}

pub fn function_signature(args: &[String]) -> Option<(&str, Vec<String>)> {
    let (name, params) = args.split_first()?;
    Some((name.as_str(), params.to_vec()))
}

pub fn call_name(args: &[String]) -> Option<&str> {
    args.first().map(String::as_str)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn function_commands_are_available_in_level_one() {
        assert!(is_available());
    }

    #[test]
    fn extracts_function_signature() {
        let args = vec!["greet".to_string(), "person".to_string()];

        assert_eq!(
            function_signature(&args),
            Some(("greet", vec!["person".to_string()]))
        );
        assert_eq!(function_signature(&[]), None);
    }

    #[test]
    fn extracts_call_name() {
        assert_eq!(call_name(&["greet".to_string()]), Some("greet"));
        assert_eq!(call_name(&[]), None);
    }
}
