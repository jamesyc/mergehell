#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MergeDriverInput {
    pub base: String,
    pub ours: String,
    pub theirs: String,
}

impl MergeDriverInput {
    pub fn new(
        base: impl Into<String>,
        ours: impl Into<String>,
        theirs: impl Into<String>,
    ) -> Self {
        Self {
            base: base.into(),
            ours: ours.into(),
            theirs: theirs.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stores_merge_driver_paths() {
        let input = MergeDriverInput::new("base.mh", "ours.mh", "theirs.mh");

        assert_eq!(input.base, "base.mh");
        assert_eq!(input.ours, "ours.mh");
        assert_eq!(input.theirs, "theirs.mh");
    }
}
