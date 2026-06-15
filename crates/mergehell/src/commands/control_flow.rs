pub fn is_available() -> bool {
    true
}

pub fn repeat_count(args: &[String]) -> Result<usize, String> {
    let Some(raw) = args.first() else {
        return Err("repeat requires a count".to_string());
    };
    raw.parse::<usize>()
        .map_err(|_| format!("repeat count must be a non-negative integer: {raw}"))
}

pub fn condition_name(args: &[String]) -> Option<&str> {
    args.first().map(String::as_str)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn control_flow_commands_are_available_in_level_one() {
        assert!(is_available());
    }

    #[test]
    fn parses_repeat_count() {
        assert_eq!(repeat_count(&["3".to_string()]), Ok(3));
        assert_eq!(
            repeat_count(&[]),
            Err("repeat requires a count".to_string())
        );
        assert_eq!(
            repeat_count(&["bad".to_string()]),
            Err("repeat count must be a non-negative integer: bad".to_string())
        );
    }

    #[test]
    fn extracts_condition_name() {
        assert_eq!(condition_name(&["enabled".to_string()]), Some("enabled"));
        assert_eq!(condition_name(&[]), None);
    }
}
