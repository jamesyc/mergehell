#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DiffMetadata {
    pub source: Option<String>,
    pub target: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PatchLine<'a> {
    Added(&'a str),
    Removed(&'a str),
    Context(&'a str),
    Other(&'a str),
}

pub fn looks_like_patch(text: &str) -> bool {
    text.lines().any(|line| {
        line.starts_with("diff --git ")
            || line.starts_with("diff --cc ")
            || line.starts_with("@@")
            || line.starts_with("@@@")
    })
}

pub fn classify_patch_line(text: &str) -> PatchLine<'_> {
    if text.starts_with("+++") || text.starts_with("---") {
        PatchLine::Other(text)
    } else if let Some(rest) = text.strip_prefix('+') {
        PatchLine::Added(rest)
    } else if let Some(rest) = text.strip_prefix('-') {
        PatchLine::Removed(rest)
    } else if let Some(rest) = text.strip_prefix(' ') {
        PatchLine::Context(rest)
    } else {
        PatchLine::Other(text)
    }
}

pub fn forward_patch_text(text: &str) -> Option<&str> {
    match classify_patch_line(text) {
        PatchLine::Added(text) | PatchLine::Context(text) | PatchLine::Other(text) => Some(text),
        PatchLine::Removed(_) => None,
    }
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

    #[test]
    fn detects_patch_like_text() {
        assert!(looks_like_patch("diff --git a/file b/file\n@@ -1 +1 @@\n"));
        assert!(looks_like_patch("@@@ -1 -1 +1 @@@ main\n"));
        assert!(!looks_like_patch(
            "<<<<<<< print\nhello\n=======\nbye\n>>>>>>> print\n"
        ));
    }

    #[test]
    fn classifies_patch_lines() {
        assert_eq!(classify_patch_line("+added"), PatchLine::Added("added"));
        assert_eq!(
            classify_patch_line("-removed"),
            PatchLine::Removed("removed")
        );
        assert_eq!(
            classify_patch_line(" context"),
            PatchLine::Context("context")
        );
        assert_eq!(
            classify_patch_line("+++ b/file"),
            PatchLine::Other("+++ b/file")
        );
        assert_eq!(classify_patch_line("raw"), PatchLine::Other("raw"));
    }

    #[test]
    fn forward_patch_text_skips_removed_lines() {
        assert_eq!(forward_patch_text("+added"), Some("added"));
        assert_eq!(forward_patch_text("-removed"), None);
        assert_eq!(forward_patch_text(" context"), Some("context"));
        assert_eq!(forward_patch_text("raw"), Some("raw"));
    }
}
