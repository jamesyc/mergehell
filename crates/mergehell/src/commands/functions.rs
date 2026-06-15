pub fn is_available() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn function_commands_are_not_level_zero_features() {
        assert!(!is_available());
    }
}
