pub fn is_available() -> bool {
    true
}

pub fn binding_name(args: &[String]) -> Option<&str> {
    args.first().map(String::as_str)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binding_commands_are_available_in_level_one() {
        assert!(is_available());
    }

    #[test]
    fn extracts_binding_name_from_first_arg() {
        let args = vec!["name".to_string()];

        assert_eq!(binding_name(&args), Some("name"));
        assert_eq!(binding_name(&[]), None);
    }
}
