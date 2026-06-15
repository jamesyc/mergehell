use crate::source::{SourceLine, Span};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LineKind {
    ConflictStart { label: String, marker_len: usize },
    ConflictBase { label: String, marker_len: usize },
    ConflictSplit { marker_len: usize },
    ConflictEnd { label: String, marker_len: usize },
    DiffGit { text: String },
    DiffCombined { text: String },
    DiffIndex { text: String },
    DiffOldFile { text: String },
    DiffNewFile { text: String },
    HunkHeader { text: String },
    CombinedHunkHeader { text: String },
    Hint { text: String },
    Status { text: String },
    NoFinalNewline,
    Raw { text: String },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClassifiedLine {
    pub kind: LineKind,
    pub span: Span,
    pub indented_marker: bool,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct LineOptions {
    pub accept_regret: bool,
    pub git_status_mode: bool,
}

pub fn classify_line(line: &SourceLine<'_>, options: LineOptions) -> ClassifiedLine {
    let (trimmed, indented) = trim_leading_space(line.text);

    if let Some((marker_len, label)) = marker(trimmed, '<', options.accept_regret) {
        return ClassifiedLine {
            kind: LineKind::ConflictStart { label, marker_len },
            span: line.span,
            indented_marker: indented,
        };
    }

    if let Some((marker_len, label)) = marker(trimmed, '|', options.accept_regret) {
        return ClassifiedLine {
            kind: LineKind::ConflictBase { label, marker_len },
            span: line.span,
            indented_marker: indented,
        };
    }

    if let Some((marker_len, label)) = marker(trimmed, '=', options.accept_regret) {
        if label.is_empty() {
            return ClassifiedLine {
                kind: LineKind::ConflictSplit { marker_len },
                span: line.span,
                indented_marker: indented,
            };
        }
    }

    if let Some((marker_len, label)) = marker(trimmed, '>', options.accept_regret) {
        return ClassifiedLine {
            kind: LineKind::ConflictEnd { label, marker_len },
            span: line.span,
            indented_marker: indented,
        };
    }

    let kind = classify_non_marker(line.text, options);
    ClassifiedLine {
        kind,
        span: line.span,
        indented_marker: false,
    }
}

fn trim_leading_space(text: &str) -> (&str, bool) {
    let trimmed = text.trim_start_matches(|ch| ch == ' ' || ch == '\t');
    (trimmed, trimmed.len() != text.len())
}

fn marker(text: &str, marker: char, accept_regret: bool) -> Option<(usize, String)> {
    let marker_len = text.chars().take_while(|ch| *ch == marker).count();
    if marker_len == 0 {
        return None;
    }

    let recognized = marker_len == 7 || (accept_regret && marker_len >= 6);
    if !recognized {
        return None;
    }

    let rest = &text[marker_len..];
    if !rest.is_empty() && !rest.chars().next().is_some_and(char::is_whitespace) {
        return None;
    }

    Some((marker_len, rest.trim().to_string()))
}

fn classify_non_marker(text: &str, options: LineOptions) -> LineKind {
    if text.starts_with("diff --git ") {
        LineKind::DiffGit {
            text: text.to_string(),
        }
    } else if text.starts_with("diff --cc ") {
        LineKind::DiffCombined {
            text: text.to_string(),
        }
    } else if text.starts_with("index ") {
        LineKind::DiffIndex {
            text: text.to_string(),
        }
    } else if text.starts_with("--- ") {
        LineKind::DiffOldFile {
            text: text.to_string(),
        }
    } else if text.starts_with("+++ ") {
        LineKind::DiffNewFile {
            text: text.to_string(),
        }
    } else if text.starts_with("@@@") {
        LineKind::CombinedHunkHeader {
            text: text.to_string(),
        }
    } else if text.starts_with("@@") {
        LineKind::HunkHeader {
            text: text.to_string(),
        }
    } else if text.starts_with("hint:")
        || text.starts_with("error:")
        || text.starts_with("CONFLICT (")
    {
        LineKind::Hint {
            text: text.to_string(),
        }
    } else if text == r"\ No newline at end of file" {
        LineKind::NoFinalNewline
    } else if options.git_status_mode && is_status_line(text) {
        LineKind::Status {
            text: text.to_string(),
        }
    } else {
        LineKind::Raw {
            text: text.to_string(),
        }
    }
}

fn is_status_line(text: &str) -> bool {
    text.starts_with("On branch ")
        || text == "You have unmerged paths."
        || text == "Unmerged paths:"
        || text.trim_start().starts_with("both modified:")
        || text.trim_start().starts_with("deleted by us:")
        || text.trim_start().starts_with("deleted by them:")
        || text.trim_start().starts_with("added by us:")
        || text.trim_start().starts_with("added by them:")
        || text == "Untracked files:"
        || text.starts_with("nothing to commit")
        || text.starts_with("working tree clean")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::SourceFile;

    fn classify(text: &str) -> ClassifiedLine {
        let source = SourceFile::new("test.mh", text);
        classify_line(
            &source.lines().next().unwrap(),
            LineOptions {
                accept_regret: false,
                git_status_mode: true,
            },
        )
    }

    #[test]
    fn classifies_start_marker_with_label() {
        let line = classify("<<<<<<< print");

        assert_eq!(
            line.kind,
            LineKind::ConflictStart {
                label: "print".to_string(),
                marker_len: 7
            }
        );
        assert!(!line.indented_marker);
    }

    #[test]
    fn classifies_indented_marker() {
        let line = classify("  <<<<<<< print");

        assert_eq!(
            line.kind,
            LineKind::ConflictStart {
                label: "print".to_string(),
                marker_len: 7
            }
        );
        assert!(line.indented_marker);
    }

    #[test]
    fn classifies_split_marker_without_label() {
        let line = classify("=======");

        assert_eq!(line.kind, LineKind::ConflictSplit { marker_len: 7 });
    }

    #[test]
    fn equals_marker_with_label_is_raw() {
        let line = classify("======= unexpected");

        assert_eq!(
            line.kind,
            LineKind::Raw {
                text: "======= unexpected".to_string()
            }
        );
    }

    #[test]
    fn near_conflict_requires_accept_regret() {
        let source = SourceFile::new("test.mh", "<<<<<< almost");
        let line = source.lines().next().unwrap();

        assert!(matches!(
            classify_line(&line, LineOptions::default()).kind,
            LineKind::Raw { .. }
        ));
        assert!(matches!(
            classify_line(
                &line,
                LineOptions {
                    accept_regret: true,
                    git_status_mode: false
                }
            )
            .kind,
            LineKind::ConflictStart { marker_len: 6, .. }
        ));
    }

    #[test]
    fn classifies_diff_hunk_hint_status_and_no_newline() {
        assert!(matches!(
            classify("diff --git a b").kind,
            LineKind::DiffGit { .. }
        ));
        assert!(matches!(
            classify("@@ -1 +1 @@").kind,
            LineKind::HunkHeader { .. }
        ));
        assert!(matches!(
            classify("hint: prefer ours").kind,
            LineKind::Hint { .. }
        ));
        assert!(matches!(
            classify("On branch main").kind,
            LineKind::Status { .. }
        ));
        assert!(matches!(
            classify(r"\ No newline at end of file").kind,
            LineKind::NoFinalNewline
        ));
    }
}
