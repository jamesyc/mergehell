#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DiffMetadata {
    pub source: Option<String>,
    pub target: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diff_metadata_defaults_to_no_paths() {
        let metadata = DiffMetadata::default();

        assert_eq!(metadata.source, None);
        assert_eq!(metadata.target, None);
    }
}
