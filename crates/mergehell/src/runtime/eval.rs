use crate::commands::{dispatch_for, print, CommandDispatch};
use crate::diagnostic::{render_diagnostics, Diagnostic, Severity};
use crate::resolve::strategy::Strategy;
use crate::source::SourceFile;
use crate::syntax::ast::{
    ConflictNode, DiffNode, ErrorNode, HintNode, HunkNode, Node, Program, RawTextNode, StatusNode,
};
use crate::syntax::parser::{parse_source, ParseOptions};

use super::context::RuntimeContext;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RunOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

pub fn run_source(
    name: impl Into<String>,
    text: impl Into<String>,
    strategy: Strategy,
) -> RunOutput {
    let source = SourceFile::new(name, text);
    let program = parse_source(&source, ParseOptions::default());

    if program.has_errors() {
        let errors = program
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.severity == Severity::Error)
            .cloned()
            .collect::<Vec<_>>();
        return RunOutput {
            stdout: String::new(),
            stderr: render_diagnostics(&errors),
            exit_code: 1,
        };
    }

    let mut context = RuntimeContext::new();
    let eval_result = eval_program(&program, strategy, &mut context);
    let warnings = program
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity == Severity::Warning)
        .cloned()
        .collect::<Vec<_>>();
    let mut stderr = render_diagnostics(&warnings);

    match eval_result {
        Ok(()) => RunOutput {
            stdout: context.into_stdout(),
            stderr,
            exit_code: 0,
        },
        Err(diagnostics) => {
            stderr.push_str(&render_diagnostics(&diagnostics));
            RunOutput {
                stdout: context.into_stdout(),
                stderr,
                exit_code: 1,
            }
        }
    }
}

pub fn eval_program(
    program: &Program,
    strategy: Strategy,
    context: &mut RuntimeContext,
) -> Result<(), Vec<Diagnostic>> {
    eval_nodes(&program.items, strategy, context)
}

fn eval_nodes(
    nodes: &[Node],
    strategy: Strategy,
    context: &mut RuntimeContext,
) -> Result<(), Vec<Diagnostic>> {
    for node in nodes {
        eval_node(node, strategy, context)?;
    }
    Ok(())
}

fn eval_node(
    node: &Node,
    strategy: Strategy,
    context: &mut RuntimeContext,
) -> Result<(), Vec<Diagnostic>> {
    match node {
        Node::Conflict(conflict) => eval_conflict(conflict, strategy, context),
        Node::Error(error) => Err(vec![runtime_error_for_parser_node(error)]),
        Node::RawText(_) | Node::Diff(_) | Node::Hunk(_) | Node::Hint(_) | Node::Status(_) => {
            Ok(())
        }
    }
}

fn eval_conflict(
    conflict: &ConflictNode,
    strategy: Strategy,
    context: &mut RuntimeContext,
) -> Result<(), Vec<Diagnostic>> {
    let resolver = strategy.resolver();
    let selected = resolver.select(conflict);

    match dispatch_for(&conflict.command.name) {
        CommandDispatch::Print => {
            let text = render_nodes_as_text(selected.nodes, strategy)?;
            context.write(&text);
            Ok(())
        }
        CommandDispatch::Transparent => eval_nodes(selected.nodes, strategy, context),
    }
}

fn render_nodes_as_text(nodes: &[Node], strategy: Strategy) -> Result<String, Vec<Diagnostic>> {
    let mut output = String::new();
    for node in nodes {
        match node {
            Node::RawText(node) => append_source_line(&mut output, node),
            Node::Diff(node) => append_diff_line(&mut output, node),
            Node::Hunk(node) => append_hunk_line(&mut output, node),
            Node::Hint(node) => append_hint_line(&mut output, node),
            Node::Status(node) => append_status_line(&mut output, node),
            Node::Conflict(conflict) => {
                let mut nested = RuntimeContext::new();
                eval_conflict(conflict, strategy, &mut nested)?;
                output.push_str(&nested.into_stdout());
            }
            Node::Error(error) => return Err(vec![runtime_error_for_parser_node(error)]),
        }
    }
    Ok(output)
}

fn append_source_line(output: &mut String, node: &RawTextNode) {
    print::append_line(output, &node.text);
}

fn append_diff_line(output: &mut String, node: &DiffNode) {
    print::append_line(output, &node.text);
}

fn append_hunk_line(output: &mut String, node: &HunkNode) {
    print::append_line(output, &node.text);
}

fn append_hint_line(output: &mut String, node: &HintNode) {
    print::append_line(output, &node.text);
}

fn append_status_line(output: &mut String, node: &StatusNode) {
    print::append_line(output, &node.text);
}

fn runtime_error_for_parser_node(node: &ErrorNode) -> Diagnostic {
    Diagnostic::runtime_error("cannot evaluate parser error", Some(node.span))
        .with_expected_actual("valid MergeHell node", node.message.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(text: &str, strategy: Strategy) -> RunOutput {
        run_source("test.mh", text, strategy)
    }

    #[test]
    fn runs_print_with_ours_strategy() {
        let output = run(
            "<<<<<<< print\nHello, world!\n=======\nGoodbye, world!\n>>>>>>> print\n",
            Strategy::Ours,
        );

        assert_eq!(output.exit_code, 0);
        assert_eq!(output.stdout, "Hello, world!\n");
        assert_eq!(output.stderr, "");
    }

    #[test]
    fn runs_print_with_theirs_strategy() {
        let output = run(
            "<<<<<<< print\nHello, world!\n=======\nGoodbye, world!\n>>>>>>> print\n",
            Strategy::Theirs,
        );

        assert_eq!(output.exit_code, 0);
        assert_eq!(output.stdout, "Goodbye, world!\n");
    }

    #[test]
    fn raw_text_outside_conflicts_is_ignored_by_runtime() {
        let output = run(
            "outside\n<<<<<<< print\ninside\n=======\nfallback\n>>>>>>> print\n",
            Strategy::Ours,
        );

        assert_eq!(output.exit_code, 0);
        assert_eq!(output.stdout, "inside\n");
    }

    #[test]
    fn empty_print_lane_outputs_nothing() {
        let output = run(
            "<<<<<<< print\n=======\nGoodbye\n>>>>>>> print\n",
            Strategy::Ours,
        );

        assert_eq!(output.exit_code, 0);
        assert_eq!(output.stdout, "");
    }

    #[test]
    fn blank_line_in_print_lane_outputs_blank_line() {
        let output = run(
            "<<<<<<< print\n\n=======\nGoodbye\n>>>>>>> print\n",
            Strategy::Ours,
        );

        assert_eq!(output.exit_code, 0);
        assert_eq!(output.stdout, "\n");
    }

    #[test]
    fn transparent_unknown_command_evaluates_selected_lane() {
        let output = run(
            "<<<<<<< HEAD\n<<<<<<< print\nHello\n=======\nGoodbye\n>>>>>>> print\n=======\n<<<<<<< print\nHola\n=======\nAdios\n>>>>>>> print\n>>>>>>> feature/spanish\n",
            Strategy::Ours,
        );

        assert_eq!(output.exit_code, 0);
        assert_eq!(output.stdout, "Hello\n");
    }

    #[test]
    fn transparent_unknown_command_obeys_outer_theirs() {
        let output = run(
            "<<<<<<< HEAD\n<<<<<<< print\nHello\n=======\nGoodbye\n>>>>>>> print\n=======\n<<<<<<< print\nHola\n=======\nAdios\n>>>>>>> print\n>>>>>>> feature/spanish\n",
            Strategy::Theirs,
        );

        assert_eq!(output.exit_code, 0);
        assert_eq!(output.stdout, "Adios\n");
    }

    #[test]
    fn parser_errors_stop_runtime() {
        let output = run("<<<<<<< print\nHello\n", Strategy::Ours);

        assert_eq!(output.exit_code, 1);
        assert_eq!(output.stdout, "");
        assert!(output
            .stderr
            .contains("CONFLICT (syntax): Merge conflict in parser"));
    }

    #[test]
    fn parser_warnings_are_rendered_to_stderr_without_failing() {
        let output = run(
            "  <<<<<<< print\nHello\n=======\nGoodbye\n>>>>>>> print\n",
            Strategy::Ours,
        );

        assert_eq!(output.exit_code, 0);
        assert_eq!(output.stdout, "Hello\n");
        assert!(output
            .stderr
            .contains("warning: indented conflict marker detected"));
    }
}
