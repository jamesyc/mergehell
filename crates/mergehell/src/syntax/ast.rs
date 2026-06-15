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

    pub fn conflict_count(&self) -> usize {
        self.items.iter().map(Node::conflict_count).sum()
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

    pub fn conflict_count(&self) -> usize {
        match self {
            Node::Conflict(node) => {
                1 + node.ours.iter().map(Node::conflict_count).sum::<usize>()
                    + node
                        .base
                        .as_ref()
                        .map(|base| base.items.iter().map(Node::conflict_count).sum())
                        .unwrap_or(0)
                    + node.theirs.iter().map(Node::conflict_count).sum::<usize>()
            }
            Node::RawText(_)
            | Node::Diff(_)
            | Node::Hunk(_)
            | Node::Hint(_)
            | Node::Status(_)
            | Node::Error(_) => 0,
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

pub fn program_to_json(program: &Program) -> String {
    let mut json = String::new();
    json.push_str("{\"type\":\"Program\",\"conflicts\":");
    json.push_str(&program.conflict_count().to_string());
    json.push_str(",\"diagnostics\":");
    json.push_str(&program.diagnostics.len().to_string());
    json.push_str(",\"items\":[");
    for (index, item) in program.items.iter().enumerate() {
        if index > 0 {
            json.push(',');
        }
        node_to_json(item, &mut json);
    }
    json.push_str("]}");
    json
}

fn node_to_json(node: &Node, json: &mut String) {
    match node {
        Node::Conflict(node) => {
            json.push_str("{\"type\":\"Conflict\",\"command\":");
            json_string(&node.command.raw, json);
            json.push_str(",\"metadata\":");
            json_string(&node.metadata.raw, json);
            json.push_str(",\"ours\":[");
            nodes_to_json(&node.ours, json);
            json.push_str("],\"base\":");
            if let Some(base) = &node.base {
                json.push('[');
                nodes_to_json(&base.items, json);
                json.push(']');
            } else {
                json.push_str("null");
            }
            json.push_str(",\"theirs\":[");
            nodes_to_json(&node.theirs, json);
            json.push_str("]}");
        }
        Node::RawText(node) => text_node_to_json("RawText", &node.text, json),
        Node::Diff(node) => text_node_to_json("Diff", &node.text, json),
        Node::Hunk(node) => text_node_to_json("Hunk", &node.text, json),
        Node::Hint(node) => text_node_to_json("Hint", &node.text, json),
        Node::Status(node) => text_node_to_json("Status", &node.text, json),
        Node::Error(node) => text_node_to_json("Error", &node.message, json),
    }
}

fn nodes_to_json(nodes: &[Node], json: &mut String) {
    for (index, node) in nodes.iter().enumerate() {
        if index > 0 {
            json.push(',');
        }
        node_to_json(node, json);
    }
}

fn text_node_to_json(kind: &str, text: &str, json: &mut String) {
    json.push_str("{\"type\":");
    json_string(kind, json);
    json.push_str(",\"text\":");
    json_string(text, json);
    json.push('}');
}

fn json_string(value: &str, json: &mut String) {
    json.push('"');
    for ch in value.chars() {
        match ch {
            '"' => json.push_str("\\\""),
            '\\' => json.push_str("\\\\"),
            '\n' => json.push_str("\\n"),
            '\r' => json.push_str("\\r"),
            '\t' => json.push_str("\\t"),
            ch if ch.is_control() => {
                json.push_str("\\u");
                json.push_str(&format!("{:04x}", ch as u32));
            }
            ch => json.push(ch),
        }
    }
    json.push('"');
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

    #[test]
    fn counts_nested_conflicts() {
        let span = span();
        let nested = Node::Conflict(ConflictNode {
            command: CommandHead::parse("print"),
            ours: Vec::new(),
            base: None,
            theirs: Vec::new(),
            metadata: Metadata::parse("print"),
            span,
        });
        let program = Program::new(
            vec![Node::Conflict(ConflictNode {
                command: CommandHead::parse("HEAD"),
                ours: vec![nested],
                base: None,
                theirs: Vec::new(),
                metadata: Metadata::parse("feature"),
                span,
            })],
            Vec::new(),
        );

        assert_eq!(program.conflict_count(), 2);
    }

    #[test]
    fn renders_program_json() {
        let program = Program::new(
            vec![Node::RawText(RawTextNode {
                text: "hello \"json\"".to_string(),
                span: span(),
            })],
            Vec::new(),
        );

        assert_eq!(
            program_to_json(&program),
            "{\"type\":\"Program\",\"conflicts\":0,\"diagnostics\":0,\"items\":[{\"type\":\"RawText\",\"text\":\"hello \\\"json\\\"\"}]}"
        );
    }
}
