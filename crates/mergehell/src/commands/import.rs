pub fn is_available() -> bool {
    true
}

pub fn first_import_path(text: &str) -> Option<&str> {
    text.lines().map(str::trim).find(|line| !line.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn import_commands_are_available_in_phase_four() {
        assert!(is_available());
    }

    #[test]
    fn extracts_first_non_empty_import_path() {
        assert_eq!(
            first_import_path("\n module.mh\nfallback.mh\n"),
            Some("module.mh")
        );
        assert_eq!(first_import_path("\n\n"), None);
    }
}
