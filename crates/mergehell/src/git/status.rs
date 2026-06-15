#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GitStatusMode {
    Disabled,
    Enabled,
}

impl Default for GitStatusMode {
    fn default() -> Self {
        Self::Disabled
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn git_status_mode_defaults_to_disabled() {
        assert_eq!(GitStatusMode::default(), GitStatusMode::Disabled);
    }
}
