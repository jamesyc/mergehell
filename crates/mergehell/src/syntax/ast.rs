use crate::diagnostic::{Diagnostic, Severity};
use crate::source::Span;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Program {
    pub items: Vec<Node>,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Node {
    Conflict(ConflictNode),
    RawText(RawTextNode),
    Diff(DiffNode),
    Hunk(HunkNode),
    Hint(HintNode),
    Status(StatusNode),
    Error(ErrorNode),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConflictNode {
    pub command: CommandHead,
    pub ours: Vec<Node>,
    pub base: Option<Lane>,
    pub theirs: Vec<Node>,
    pub metadata: Metadata,
    pub span: Span,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Lane {
    pub label: Option<String>,
    pub items: Vec<Node>,
    pub span: Span,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommandHead {
    pub name: String,
    pub args: Vec<String>,
    pub raw: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Metadata {
    pub raw: String,
    pub tokens: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RawTextNode {
    pub text: String,
    pub span: Span,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiffNode {
    pub kind: DiffKind,
    pub text: String,
    pub span: Span,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DiffKind {
    Git,
    Combined,
    Index,
    OldFile,
    NewFile,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HunkNode {
    pub combined: bool,
    pub text: String,
    pub span: Span,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HintNode {
    pub text: String,
    pub span: Span,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StatusNode {
    pub text: String,
    pub span: Span,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ErrorNode {
    pub message: String,
    pub span: Span,
}

impl Program {
    pub fn new(items: Vec<Node>, diagnostics: Vec<Diagnostic>) -> Self {
        Self { items, diagnostics }
    }

    pub fn has_conflicts(&self) -> bool {
        self.items.iter().any(Node::has_conflicts)
    }

    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == Severity::Error)
    }
}

impl Node {
    pub fn has_conflicts(&self) -> bool {
        match self {
            Node::Conflict(_) => true,
            Node::RawText(_)
            | Node::Diff(_)
            | Node::Hunk(_)
            | Node::Hint(_)
            | Node::Status(_)
            | Node::Error(_) => false,
        }
    }

    pub fn span(&self) -> Span {
        match self {
            Node::Conflict(node) => node.span,
            Node::RawText(node) => node.span,
            Node::Diff(node) => node.span,
            Node::Hunk(node) => node.span,
            Node::Hint(node) => node.span,
            Node::Status(node) => node.span,
            Node::Error(node) => node.span,
        }
    }
}

impl CommandHead {
    pub fn parse(raw: impl Into<String>) -> Self {
        let raw = raw.into();
        let mut parts = raw.split_whitespace();
        let name = parts.next().unwrap_or("").to_string();
        let args = parts.map(str::to_string).collect();

        Self { name, args, raw }
    }
}

impl Metadata {
    pub fn parse(raw: impl Into<String>) -> Self {
        let raw = raw.into();
        let tokens = raw.split_whitespace().map(str::to_string).collect();

        Self { raw, tokens }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn span() -> Span {
        Span {
            file_id: 0,
            start: 0,
            end: 1,
        }
    }

    #[test]
    fn parses_command_head_with_args() {
        let command = CommandHead::parse("print greeting loudly");

        assert_eq!(command.name, "print");
        assert_eq!(
            command.args,
            vec!["greeting".to_string(), "loudly".to_string()]
        );
        assert_eq!(command.raw, "print greeting loudly");
    }

    #[test]
    fn parses_empty_command_head() {
        let command = CommandHead::parse("");

        assert_eq!(command.name, "");
        assert!(command.args.is_empty());
    }

    #[test]
    fn parses_metadata_tokens() {
        let metadata = Metadata::parse("feature/cache retry/3");

        assert_eq!(metadata.raw, "feature/cache retry/3");
        assert_eq!(
            metadata.tokens,
            vec!["feature/cache".to_string(), "retry/3".to_string()]
        );
    }

    #[test]
    fn program_reports_conflicts() {
        let program = Program::new(
            vec![Node::Conflict(ConflictNode {
                command: CommandHead::parse("print"),
                ours: Vec::new(),
                base: None,
                theirs: Vec::new(),
                metadata: Metadata::parse("print"),
                span: span(),
            })],
            Vec::new(),
        );

        assert!(program.has_conflicts());
        assert!(!program.has_errors());
    }

    #[test]
    fn raw_program_has_no_conflicts() {
        let program = Program::new(
            vec![Node::RawText(RawTextNode {
                text: "hello".to_string(),
                span: span(),
            })],
            Vec::new(),
        );

        assert!(!program.has_conflicts());
    }
}
