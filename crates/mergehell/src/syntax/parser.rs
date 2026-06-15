use crate::diagnostic::Diagnostic;
use crate::source::{SourceFile, Span};

use super::ast::{
    CommandHead, ConflictNode, DiffKind, DiffNode, ErrorNode, HintNode, HunkNode, Lane, Metadata,
    Node, Program, RawTextNode, StatusNode,
};
use super::line::{classify_line, ClassifiedLine, LineKind, LineOptions};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ParseOptions {
    pub accept_regret: bool,
    pub git_status_mode: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum LaneState {
    Ours,
    Base,
    Theirs,
}

#[derive(Debug)]
struct Frame {
    command: CommandHead,
    start_span: Span,
    ours: Vec<Node>,
    base: Option<LaneBuilder>,
    theirs: Vec<Node>,
    current: LaneState,
}

#[derive(Debug)]
struct LaneBuilder {
    label: Option<String>,
    start_span: Span,
    items: Vec<Node>,
}

pub fn parse_source(source: &SourceFile, options: ParseOptions) -> Program {
    let parser = Parser {
        source,
        options,
        root: Vec::new(),
        stack: Vec::new(),
        diagnostics: Vec::new(),
    };
    parser.parse()
}

struct Parser<'a> {
    source: &'a SourceFile,
    options: ParseOptions,
    root: Vec<Node>,
    stack: Vec<Frame>,
    diagnostics: Vec<Diagnostic>,
}

impl Parser<'_> {
    fn parse(mut self) -> Program {
        for line in self.source.lines() {
            let classified = classify_line(
                &line,
                LineOptions {
                    accept_regret: self.options.accept_regret,
                    git_status_mode: self.options.git_status_mode,
                },
            );
            self.handle_line(classified);
        }

        while let Some(frame) = self.stack.pop() {
            let message = "Merge conflict in parser";
            self.diagnostics.push(
                Diagnostic::syntax_error(message, Some(frame.start_span))
                    .with_expected_actual(">>>>>>>", "end of file"),
            );
            self.push_node(Node::Error(ErrorNode {
                message: "unclosed conflict".to_string(),
                span: frame.start_span.join(self.source.span()),
            }));
        }

        Program::new(self.root, self.diagnostics)
    }

    fn handle_line(&mut self, line: ClassifiedLine) {
        if line.indented_marker {
            self.diagnostics.push(
                Diagnostic::syntax_warning(
                    "warning: indented conflict marker detected",
                    Some(line.span),
                )
                .with_hint("hint: you may be using YAML, which is already a cry for help"),
            );
        }

        match line.kind {
            LineKind::ConflictStart { label, .. } => self.start_conflict(label, line.span),
            LineKind::ConflictBase { label, .. } => self.start_base(label, line.span),
            LineKind::ConflictSplit { .. } => self.start_theirs(line.span),
            LineKind::ConflictEnd { label, .. } => self.end_conflict(label, line.span),
            kind => self.push_node(node_from_line(kind, line.span)),
        }
    }

    fn start_conflict(&mut self, label: String, span: Span) {
        self.stack.push(Frame {
            command: CommandHead::parse(label),
            start_span: span,
            ours: Vec::new(),
            base: None,
            theirs: Vec::new(),
            current: LaneState::Ours,
        });
    }

    fn start_base(&mut self, label: String, span: Span) {
        match self.stack.last_mut() {
            Some(frame) if frame.current == LaneState::Ours && frame.base.is_none() => {
                frame.current = LaneState::Base;
                frame.base = Some(LaneBuilder {
                    label: if label.is_empty() { None } else { Some(label) },
                    start_span: span,
                    items: Vec::new(),
                });
            }
            Some(frame) => {
                self.diagnostics.push(Diagnostic::syntax_error(
                    "misplaced base marker",
                    Some(span),
                ));
                push_to_frame(
                    frame,
                    Node::Error(ErrorNode {
                        message: "misplaced base marker".to_string(),
                        span,
                    }),
                );
            }
            None => self.unexpected_marker("base", span),
        }
    }

    fn start_theirs(&mut self, span: Span) {
        match self.stack.last_mut() {
            Some(frame) if frame.current == LaneState::Ours || frame.current == LaneState::Base => {
                frame.current = LaneState::Theirs;
            }
            Some(frame) => {
                self.diagnostics.push(Diagnostic::syntax_error(
                    "duplicate split marker",
                    Some(span),
                ));
                push_to_frame(
                    frame,
                    Node::Error(ErrorNode {
                        message: "duplicate split marker".to_string(),
                        span,
                    }),
                );
            }
            None => self.unexpected_marker("split", span),
        }
    }

    fn end_conflict(&mut self, label: String, span: Span) {
        let Some(frame) = self.stack.pop() else {
            self.unexpected_marker("end", span);
            return;
        };

        if frame.current != LaneState::Theirs {
            self.diagnostics.push(
                Diagnostic::syntax_error("missing split marker before conflict end", Some(span))
                    .with_expected_actual("=======", ">>>>>>>"),
            );
        }

        let base = frame.base.map(|builder| Lane {
            label: builder.label,
            span: builder.start_span.join(span),
            items: builder.items,
        });
        let conflict = Node::Conflict(ConflictNode {
            command: frame.command,
            ours: frame.ours,
            base,
            theirs: frame.theirs,
            metadata: Metadata::parse(label),
            span: frame.start_span.join(span),
        });
        self.push_node(conflict);
    }

    fn unexpected_marker(&mut self, marker: &str, span: Span) {
        let message = format!("unexpected {marker} marker");
        self.diagnostics
            .push(Diagnostic::syntax_error(message.clone(), Some(span)));
        self.push_node(Node::Error(ErrorNode { message, span }));
    }

    fn push_node(&mut self, node: Node) {
        if let Some(frame) = self.stack.last_mut() {
            push_to_frame(frame, node);
        } else {
            self.root.push(node);
        }
    }
}

fn push_to_frame(frame: &mut Frame, node: Node) {
    match frame.current {
        LaneState::Ours => frame.ours.push(node),
        LaneState::Base => {
            if let Some(base) = &mut frame.base {
                base.items.push(node);
            }
        }
        LaneState::Theirs => frame.theirs.push(node),
    }
}

fn node_from_line(kind: LineKind, span: Span) -> Node {
    match kind {
        LineKind::DiffGit { text } => Node::Diff(DiffNode {
            kind: DiffKind::Git,
            text,
            span,
        }),
        LineKind::DiffCombined { text } => Node::Diff(DiffNode {
            kind: DiffKind::Combined,
            text,
            span,
        }),
        LineKind::DiffIndex { text } => Node::Diff(DiffNode {
            kind: DiffKind::Index,
            text,
            span,
        }),
        LineKind::DiffOldFile { text } => Node::Diff(DiffNode {
            kind: DiffKind::OldFile,
            text,
            span,
        }),
        LineKind::DiffNewFile { text } => Node::Diff(DiffNode {
            kind: DiffKind::NewFile,
            text,
            span,
        }),
        LineKind::HunkHeader { text } => Node::Hunk(HunkNode {
            combined: false,
            text,
            span,
        }),
        LineKind::CombinedHunkHeader { text } => Node::Hunk(HunkNode {
            combined: true,
            text,
            span,
        }),
        LineKind::Hint { text } => Node::Hint(HintNode { text, span }),
        LineKind::Status { text } => Node::Status(StatusNode { text, span }),
        LineKind::NoFinalNewline => Node::RawText(RawTextNode {
            text: r"\ No newline at end of file".to_string(),
            span,
        }),
        LineKind::Raw { text } => Node::RawText(RawTextNode { text, span }),
        LineKind::ConflictStart { .. }
        | LineKind::ConflictBase { .. }
        | LineKind::ConflictSplit { .. }
        | LineKind::ConflictEnd { .. } => {
            unreachable!("marker lines are handled before node conversion")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostic::Severity;

    fn parse(text: &str) -> Program {
        parse_source(&SourceFile::new("test.mh", text), ParseOptions::default())
    }

    #[test]
    fn parses_basic_conflict() {
        let program = parse("<<<<<<< print\nHello\n=======\nGoodbye\n>>>>>>> feature/greeting\n");

        assert_eq!(program.diagnostics, Vec::new());
        let Node::Conflict(conflict) = &program.items[0] else {
            panic!("expected conflict");
        };
        assert_eq!(conflict.command.name, "print");
        assert_eq!(conflict.ours.len(), 1);
        assert_eq!(conflict.theirs.len(), 1);
        assert_eq!(
            conflict.metadata.tokens,
            vec!["feature/greeting".to_string()]
        );
    }

    #[test]
    fn preserves_base_lane() {
        let program = parse("<<<<<<< let name\nJames\n||||||| string default\nUser\n=======\nEnv\n>>>>>>> feature/env\n");

        let Node::Conflict(conflict) = &program.items[0] else {
            panic!("expected conflict");
        };
        let base = conflict.base.as_ref().expect("base lane");
        assert_eq!(base.label.as_deref(), Some("string default"));
        assert_eq!(base.items.len(), 1);
    }

    #[test]
    fn parses_nested_conflicts_with_stack() {
        let program = parse(
            "<<<<<<< HEAD\n<<<<<<< print\nHello\n=======\nGoodbye\n>>>>>>> print\n=======\n<<<<<<< print\nHola\n=======\nAdios\n>>>>>>> print\n>>>>>>> feature/spanish\n",
        );

        assert!(program.diagnostics.is_empty());
        let Node::Conflict(outer) = &program.items[0] else {
            panic!("expected outer conflict");
        };
        assert_eq!(outer.command.name, "HEAD");
        assert!(matches!(outer.ours[0], Node::Conflict(_)));
        assert!(matches!(outer.theirs[0], Node::Conflict(_)));
    }

    #[test]
    fn preserves_diff_metadata_as_nodes() {
        let program = parse("diff --git a/stdin b/stdout\nindex deadbee..c0ffee0\n--- a/stdin\n+++ b/stdout\n@@ -1 +1 @@ main\n<<<<<<< print\nHello\n=======\nGoodbye\n>>>>>>> print\n");

        assert!(matches!(program.items[0], Node::Diff(_)));
        assert!(matches!(program.items[1], Node::Diff(_)));
        assert!(matches!(program.items[2], Node::Diff(_)));
        assert!(matches!(program.items[3], Node::Diff(_)));
        assert!(matches!(program.items[4], Node::Hunk(_)));
        assert!(matches!(program.items[5], Node::Conflict(_)));
    }

    #[test]
    fn clean_text_is_raw_nodes_without_diagnostics() {
        let program = parse("hello\nworld\n");

        assert_eq!(program.items.len(), 2);
        assert!(!program.has_conflicts());
        assert!(program.diagnostics.is_empty());
    }

    #[test]
    fn unclosed_conflict_recovers_with_error_node() {
        let program = parse("<<<<<<< print\nHello\n");

        assert_eq!(program.diagnostics.len(), 1);
        assert_eq!(program.diagnostics[0].severity, Severity::Error);
        assert!(matches!(program.items[0], Node::Error(_)));
    }

    #[test]
    fn unexpected_split_recovers_with_error_node() {
        let program = parse("=======\n");

        assert_eq!(program.diagnostics.len(), 1);
        assert!(matches!(program.items[0], Node::Error(_)));
    }

    #[test]
    fn indented_marker_warns_but_parses() {
        let program = parse("  <<<<<<< print\nHello\n=======\nGoodbye\n>>>>>>> print\n");

        assert_eq!(program.diagnostics.len(), 1);
        assert_eq!(program.diagnostics[0].severity, Severity::Warning);
        assert!(program.has_conflicts());
    }

    #[test]
    fn missing_split_before_end_is_a_parser_error() {
        let program = parse("<<<<<<< print\nHello\n>>>>>>> print\n");

        assert_eq!(program.diagnostics.len(), 1);
        assert_eq!(
            program.diagnostics[0].message,
            "missing split marker before conflict end"
        );
        assert!(program.has_conflicts());
    }
}
