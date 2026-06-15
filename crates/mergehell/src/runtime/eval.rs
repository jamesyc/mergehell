use std::fs;
use std::path::PathBuf;

use crate::commands::{
    binding, control_flow, dispatch_for, functions, import, print, stdlib, CommandDispatch,
};
use crate::diagnostic::{render_diagnostics, Diagnostic, Severity};
use crate::resolve::strategy::{
    lanes_in_order, BaseResolver, OursResolver, Resolver, SelectedLane, Strategy, TheirsResolver,
};
use crate::source::SourceFile;
use crate::syntax::ast::{
    ConflictNode, DiffNode, ErrorNode, HintNode, HunkNode, Node, Program, RawTextNode, StatusNode,
};
use crate::syntax::parser::{parse_source, ParseOptions};

use super::context::{Function, RuntimeContext};
use super::control::EvalOutcome;
use super::value::Value;

type EvalResult = Result<EvalOutcome, Vec<Diagnostic>>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RunOptions {
    pub strategy: Strategy,
    pub seed: u64,
    pub parse_options: ParseOptions,
    pub strict: bool,
    pub patch_mode: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RunOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

impl Default for RunOptions {
    fn default() -> Self {
        Self {
            strategy: Strategy::Ours,
            seed: 0,
            parse_options: ParseOptions::default(),
            strict: false,
            patch_mode: false,
        }
    }
}

pub fn run_source(
    name: impl Into<String>,
    text: impl Into<String>,
    strategy: Strategy,
) -> RunOutput {
    run_source_with_options(
        name,
        text,
        RunOptions {
            strategy,
            ..RunOptions::default()
        },
    )
}

pub fn run_source_with_options(
    name: impl Into<String>,
    text: impl Into<String>,
    options: RunOptions,
) -> RunOutput {
    let name = name.into();
    let text = text.into();
    let options = RunOptions {
        patch_mode: options.patch_mode || crate::git::diff::looks_like_patch(&text),
        ..options
    };
    let source = SourceFile::new(name.clone(), text);
    let program = parse_source(&source, options.parse_options);

    if program.has_errors()
        || (options.strict
            && program
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.severity == Severity::Warning))
    {
        let errors = program
            .diagnostics
            .iter()
            .filter(|diagnostic| {
                diagnostic.severity == Severity::Error
                    || (options.strict && diagnostic.severity == Severity::Warning)
            })
            .cloned()
            .collect::<Vec<_>>();
        return RunOutput {
            stdout: String::new(),
            stderr: render_diagnostics(&errors),
            exit_code: 1,
        };
    }

    let mut context = RuntimeContext::new(options.seed).with_source_name(&name);
    let eval_result = eval_program(&program, options, &mut context);
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
    options: RunOptions,
    context: &mut RuntimeContext,
) -> Result<(), Vec<Diagnostic>> {
    eval_nodes(&program.items, options, context).map(|_| ())
}

fn eval_nodes(nodes: &[Node], options: RunOptions, context: &mut RuntimeContext) -> EvalResult {
    for node in nodes {
        let outcome = eval_node(node, options, context)?;
        if !outcome.is_unit() {
            return Ok(outcome);
        }
    }
    Ok(EvalOutcome::Unit)
}

fn eval_node(node: &Node, options: RunOptions, context: &mut RuntimeContext) -> EvalResult {
    match node {
        Node::Conflict(conflict) => eval_conflict(conflict, options, context),
        Node::Error(error) => Err(vec![runtime_error_for_parser_node(error)]),
        Node::Status(status) => {
            record_status_metadata(status, context);
            Ok(EvalOutcome::Unit)
        }
        Node::RawText(_) | Node::Diff(_) | Node::Hunk(_) | Node::Hint(_) => Ok(EvalOutcome::Unit),
    }
}

fn eval_conflict(
    conflict: &ConflictNode,
    options: RunOptions,
    context: &mut RuntimeContext,
) -> EvalResult {
    match dispatch_for(&conflict.command.name) {
        CommandDispatch::Print => eval_print(conflict, options, context),
        CommandDispatch::Let => eval_let(conflict, options, context),
        CommandDispatch::If => eval_if(conflict, options, context),
        CommandDispatch::Repeat => eval_repeat(conflict, options, context),
        CommandDispatch::Function => eval_function(conflict, options, context),
        CommandDispatch::Call => eval_call(conflict, options, context),
        CommandDispatch::Return => eval_return(conflict, options, context),
        CommandDispatch::Try => eval_try(conflict, options, context),
        CommandDispatch::Throw => eval_throw(conflict, options, context),
        CommandDispatch::Import => eval_import(conflict, options, context),
        CommandDispatch::Resolve => eval_resolve(conflict, options, context),
        CommandDispatch::Transparent => eval_transparent(conflict, options, context),
    }
}

fn eval_print(
    conflict: &ConflictNode,
    options: RunOptions,
    context: &mut RuntimeContext,
) -> EvalResult {
    for lane in selected_lanes(conflict, options, context)? {
        let text = render_nodes_as_text(lane.nodes, options, context)?;
        context.write(&text);
    }
    Ok(EvalOutcome::Unit)
}

fn eval_let(
    conflict: &ConflictNode,
    options: RunOptions,
    context: &mut RuntimeContext,
) -> EvalResult {
    let Some(name) = binding::binding_name(&conflict.command.args) else {
        return Err(vec![runtime_error("let requires a binding name", conflict)]);
    };
    let expected_type = expected_let_type(conflict);

    for lane in selected_lanes(conflict, options, context)? {
        let text = render_nodes_as_text(lane.nodes, options, context)?;
        let value = Value::parse_text(&text);
        if let Some(expected_type) = expected_type {
            validate_type(name, expected_type, &value, conflict)?;
        }
        context.set_var(name, value);
    }
    Ok(EvalOutcome::Unit)
}

fn eval_if(
    conflict: &ConflictNode,
    options: RunOptions,
    context: &mut RuntimeContext,
) -> EvalResult {
    let Some(condition) = control_flow::condition_name(&conflict.command.args) else {
        return Err(vec![runtime_error("if requires a condition", conflict)]);
    };
    let selected = if condition_is_truthy(condition, context) {
        &conflict.ours
    } else {
        &conflict.theirs
    };

    eval_nodes(selected, options, context)
}

fn eval_repeat(
    conflict: &ConflictNode,
    options: RunOptions,
    context: &mut RuntimeContext,
) -> EvalResult {
    let count = control_flow::repeat_count(&conflict.command.args)
        .map_err(|message| vec![runtime_error(message, conflict)])?;

    for lane in selected_lanes(conflict, options, context)? {
        for _ in 0..count {
            let outcome = eval_nodes(lane.nodes, options, context)?;
            if !outcome.is_unit() {
                return Ok(outcome);
            }
        }
    }
    Ok(EvalOutcome::Unit)
}

fn eval_function(
    conflict: &ConflictNode,
    options: RunOptions,
    context: &mut RuntimeContext,
) -> EvalResult {
    let Some((name, params)) = functions::function_signature(&conflict.command.args) else {
        return Err(vec![runtime_error("function requires a name", conflict)]);
    };

    for lane in selected_lanes(conflict, options, context)? {
        context.define_function(
            name,
            Function {
                params: params.clone(),
                body: lane.nodes.to_vec(),
            },
        );
    }
    Ok(EvalOutcome::Unit)
}

fn eval_call(
    conflict: &ConflictNode,
    options: RunOptions,
    context: &mut RuntimeContext,
) -> EvalResult {
    let Some(name) = functions::call_name(&conflict.command.args) else {
        return Err(vec![runtime_error(
            "call requires a function name",
            conflict,
        )]);
    };
    let Some(function) = context.get_function(name).cloned() else {
        return Err(vec![runtime_error(
            format!("unknown function `{name}`"),
            conflict,
        )]);
    };

    for lane in selected_lanes(conflict, options, context)? {
        let args = call_args(lane.nodes, options, context)?;
        context.push_scope();
        for (index, param) in function.params.iter().enumerate() {
            let value = args.get(index).cloned().unwrap_or(Value::Null);
            context.set_var(param, value);
        }
        let result = eval_nodes(&function.body, options, context);
        context.pop_scope();
        match result? {
            EvalOutcome::Unit | EvalOutcome::Return(_) => {}
        }
    }
    Ok(EvalOutcome::Unit)
}

fn eval_return(
    conflict: &ConflictNode,
    options: RunOptions,
    context: &mut RuntimeContext,
) -> EvalResult {
    let mut lanes = selected_lanes(conflict, options, context)?;
    let lane = lanes.remove(0);
    let text = render_nodes_as_text(lane.nodes, options, context)?;
    Ok(EvalOutcome::Return(Value::parse_text(&text)))
}

fn eval_try(
    conflict: &ConflictNode,
    options: RunOptions,
    context: &mut RuntimeContext,
) -> EvalResult {
    let attempt = eval_nodes(&conflict.ours, options, context);
    let outcome = match attempt {
        Ok(outcome) => Ok(outcome),
        Err(_) => eval_nodes(&conflict.theirs, options, context),
    };

    if let Some(base) = &conflict.base {
        let cleanup = eval_nodes(&base.items, options, context)?;
        if !cleanup.is_unit() {
            return Ok(cleanup);
        }
    }

    outcome
}

fn eval_throw(
    conflict: &ConflictNode,
    options: RunOptions,
    context: &mut RuntimeContext,
) -> EvalResult {
    let mut lanes = selected_lanes(conflict, options, context)?;
    let lane = lanes.remove(0);
    let text = render_nodes_as_text(lane.nodes, options, context)?;
    Err(vec![Diagnostic::runtime_error(
        "Merge conflict in execution",
        Some(conflict.span),
    )
    .with_expected_actual("success", text.trim())])
}

fn eval_import(
    conflict: &ConflictNode,
    options: RunOptions,
    context: &mut RuntimeContext,
) -> EvalResult {
    for lane in selected_lanes(conflict, options, context)? {
        let text = render_nodes_as_text(lane.nodes, options, context)?;
        let Some(raw_path) = import::first_import_path(&text) else {
            return Err(vec![runtime_error("import requires a path", conflict)]);
        };
        if let Some(module) = stdlib::find_module(raw_path) {
            context.set_metadata(format!("stdlib.{}.loaded", module.name), "true");
            context.set_var(format!("stdlib.{}.loaded", module.name), Value::Bool(true));
        } else {
            eval_import_path(raw_path, options, context, conflict)?;
        }
    }
    Ok(EvalOutcome::Unit)
}

fn eval_import_path(
    raw_path: &str,
    options: RunOptions,
    context: &mut RuntimeContext,
    conflict: &ConflictNode,
) -> EvalResult {
    let path = context.resolve_import_path(raw_path);
    let import_key = path.canonicalize().unwrap_or_else(|_| path.clone());
    if !context.enter_import(import_key.clone()) {
        return Err(vec![runtime_error(
            format!("import cycle detected: {}", import_key.display()),
            conflict,
        )]);
    }

    let result = read_and_eval_import(path, options, context, conflict);
    context.leave_import();
    result
}

fn read_and_eval_import(
    path: PathBuf,
    options: RunOptions,
    context: &mut RuntimeContext,
    conflict: &ConflictNode,
) -> EvalResult {
    let source_text = fs::read_to_string(&path).map_err(|error| {
        vec![runtime_error(
            format!("could not import {}: {error}", path.display()),
            conflict,
        )]
    })?;
    let previous_dir = context.replace_current_dir(
        path.parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .map(|parent| parent.to_path_buf()),
    );
    let source = SourceFile::new(path.display().to_string(), source_text);
    let program = parse_source(&source, options.parse_options);
    let result = if program.has_errors() {
        Err(program
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.severity == Severity::Error)
            .cloned()
            .collect::<Vec<_>>())
    } else {
        eval_nodes(&program.items, options, context)
    };
    context.replace_current_dir(previous_dir);
    result
}

fn eval_resolve(
    conflict: &ConflictNode,
    options: RunOptions,
    context: &mut RuntimeContext,
) -> EvalResult {
    let Some(strategy_name) = conflict.command.args.first() else {
        return Err(vec![runtime_error("resolve requires a strategy", conflict)]);
    };
    let strategy = strategy_name
        .parse::<Strategy>()
        .map_err(|message| vec![runtime_error(message, conflict)])?;
    let override_options = RunOptions {
        strategy,
        ..options
    };

    for lane in selected_lanes(conflict, options, context)? {
        let outcome = eval_nodes(lane.nodes, override_options, context)?;
        if !outcome.is_unit() {
            return Ok(outcome);
        }
    }
    Ok(EvalOutcome::Unit)
}

fn eval_transparent(
    conflict: &ConflictNode,
    options: RunOptions,
    context: &mut RuntimeContext,
) -> EvalResult {
    for lane in selected_lanes(conflict, options, context)? {
        let outcome = eval_nodes(lane.nodes, options, context)?;
        if !outcome.is_unit() {
            return Ok(outcome);
        }
    }
    Ok(EvalOutcome::Unit)
}

fn selected_lanes<'a>(
    conflict: &'a ConflictNode,
    options: RunOptions,
    context: &mut RuntimeContext,
) -> Result<Vec<SelectedLane<'a>>, Vec<Diagnostic>> {
    match options.strategy {
        Strategy::Ours => Ok(vec![OursResolver
            .select(conflict)
            .expect("ours lane exists")]),
        Strategy::Theirs => Ok(vec![TheirsResolver
            .select(conflict)
            .expect("theirs lane exists")]),
        Strategy::Base => BaseResolver
            .select(conflict)
            .map(|lane| vec![lane])
            .ok_or_else(|| vec![no_base_diagnostic(conflict)]),
        Strategy::Union => Ok(lanes_in_order(conflict)),
        Strategy::Random => {
            let mut lanes = lanes_in_order(conflict);
            let index = context.choose_index(lanes.len()).unwrap_or(0);
            Ok(vec![lanes.remove(index)])
        }
        Strategy::Git => {
            let strategy = crate::git::status::strategy_from_current_repo()
                .map_err(|error| vec![git_strategy_diagnostic(error, conflict)])?;
            selected_lanes(
                conflict,
                RunOptions {
                    strategy,
                    ..options
                },
                context,
            )
        }
    }
}

fn render_nodes_as_text(
    nodes: &[Node],
    options: RunOptions,
    context: &mut RuntimeContext,
) -> Result<String, Vec<Diagnostic>> {
    let mut output = String::new();
    for node in nodes {
        match node {
            Node::RawText(node) => append_source_line(&mut output, node, options, context),
            Node::Diff(node) => append_diff_line(&mut output, node, context),
            Node::Hunk(node) => append_hunk_line(&mut output, node, context),
            Node::Hint(node) => append_hint_line(&mut output, node, context),
            Node::Status(node) => append_status_line(&mut output, node, context),
            Node::Conflict(conflict) => {
                let captured = context.capture_output(|context| {
                    eval_conflict(conflict, options, context).map(|_| ())
                })?;
                output.push_str(&captured);
            }
            Node::Error(error) => return Err(vec![runtime_error_for_parser_node(error)]),
        }
    }
    Ok(output)
}

fn append_source_line(
    output: &mut String,
    node: &RawTextNode,
    options: RunOptions,
    context: &RuntimeContext,
) {
    if options.patch_mode {
        if let Some(text) = crate::git::diff::forward_patch_text(&node.text) {
            print::append_line(output, &interpolate(text, context));
        }
    } else {
        print::append_line(output, &interpolate(&node.text, context));
    }
}

fn append_diff_line(output: &mut String, node: &DiffNode, context: &RuntimeContext) {
    print::append_line(output, &interpolate(&node.text, context));
}

fn append_hunk_line(output: &mut String, node: &HunkNode, context: &RuntimeContext) {
    print::append_line(output, &interpolate(&node.text, context));
}

fn append_hint_line(output: &mut String, node: &HintNode, context: &RuntimeContext) {
    print::append_line(output, &interpolate(&node.text, context));
}

fn append_status_line(output: &mut String, node: &StatusNode, context: &RuntimeContext) {
    print::append_line(output, &interpolate(&node.text, context));
}

fn record_status_metadata(node: &StatusNode, context: &mut RuntimeContext) {
    if let Some((key, value)) = crate::git::status::runtime_metadata_for_status_line(&node.text) {
        context.set_metadata(key, value.clone());
        context.set_var(key, Value::String(value));
    }
}

fn call_args(
    nodes: &[Node],
    options: RunOptions,
    context: &mut RuntimeContext,
) -> Result<Vec<Value>, Vec<Diagnostic>> {
    let text = render_nodes_as_text(nodes, options, context)?;
    Ok(text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(Value::parse_text)
        .collect())
}

fn condition_is_truthy(condition: &str, context: &RuntimeContext) -> bool {
    if let Some(value) = context.get_var(condition) {
        return value.is_truthy();
    }
    Value::parse_text(condition).is_truthy()
}

fn interpolate(text: &str, context: &RuntimeContext) -> String {
    let mut output = String::new();
    let mut remaining = text;

    while let Some(start) = remaining.find("${") {
        output.push_str(&remaining[..start]);
        let after_start = &remaining[start + 2..];
        let Some(end) = after_start.find('}') else {
            output.push_str(&remaining[start..]);
            return output;
        };
        let name = &after_start[..end];
        if let Some(value) = context.get_var(name) {
            output.push_str(&value.as_output_text());
        }
        remaining = &after_start[end + 1..];
    }

    output.push_str(remaining);
    output
}

fn expected_let_type(conflict: &ConflictNode) -> Option<&str> {
    conflict
        .base
        .as_ref()
        .and_then(|base| base.label.as_deref())
        .and_then(|label| label.split_whitespace().next())
        .filter(|name| matches!(*name, "int" | "float" | "string" | "bool"))
}

fn validate_type(
    binding_name: &str,
    expected_type: &str,
    value: &Value,
    conflict: &ConflictNode,
) -> Result<(), Vec<Diagnostic>> {
    let matches = match expected_type {
        "int" => matches!(value, Value::Number(number) if number.fract() == 0.0),
        "float" => matches!(value, Value::Number(_)),
        "string" => matches!(value, Value::String(_)),
        "bool" => matches!(value, Value::Bool(_)),
        _ => true,
    };

    if matches {
        Ok(())
    } else {
        Err(vec![Diagnostic::type_error(
            format!("Merge conflict in {binding_name}"),
            Some(conflict.span),
        )
        .with_expected_actual(expected_type, value.type_name())])
    }
}

fn runtime_error(message: impl Into<String>, conflict: &ConflictNode) -> Diagnostic {
    Diagnostic::runtime_error(message, Some(conflict.span))
}

fn no_base_diagnostic(conflict: &ConflictNode) -> Diagnostic {
    Diagnostic::runtime_error("error: no common ancestor found", Some(conflict.span))
        .with_hint("hint: manufacture a past and try again")
}

fn git_strategy_diagnostic(
    error: crate::git::status::GitStrategyError,
    conflict: &ConflictNode,
) -> Diagnostic {
    let diagnostic = Diagnostic::runtime_error(error.message, Some(conflict.span));
    if let Some(hint) = error.hint {
        diagnostic.with_hint(hint)
    } else {
        diagnostic
    }
}

fn runtime_error_for_parser_node(node: &ErrorNode) -> Diagnostic {
    Diagnostic::runtime_error("cannot evaluate parser error", Some(node.span))
        .with_expected_actual("valid MergeHell node", node.message.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn run(text: &str, strategy: Strategy) -> RunOutput {
        run_source("test.mh", text, strategy)
    }

    fn run_named(name: &str, text: &str, strategy: Strategy) -> RunOutput {
        run_source(name, text, strategy)
    }

    fn run_seeded(text: &str, strategy: Strategy, seed: u64) -> RunOutput {
        run_source_with_options(
            "test.mh",
            text,
            RunOptions {
                strategy,
                seed,
                parse_options: ParseOptions::default(),
                strict: false,
                patch_mode: false,
            },
        )
    }

    fn run_with_parse_options(text: &str, parse_options: ParseOptions) -> RunOutput {
        run_source_with_options(
            "test.mh",
            text,
            RunOptions {
                strategy: Strategy::Ours,
                seed: 0,
                parse_options,
                strict: false,
                patch_mode: false,
            },
        )
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
    fn runs_print_with_base_strategy_when_base_exists() {
        let output = run(
            "<<<<<<< print\nours\n||||||| base\nbase\n=======\ntheirs\n>>>>>>> print\n",
            Strategy::Base,
        );

        assert_eq!(output.exit_code, 0);
        assert_eq!(output.stdout, "base\n");
    }

    #[test]
    fn base_strategy_errors_when_base_is_missing() {
        let output = run(
            "<<<<<<< print\nours\n=======\ntheirs\n>>>>>>> print\n",
            Strategy::Base,
        );

        assert_eq!(output.exit_code, 1);
        assert_eq!(output.stdout, "");
        assert!(output.stderr.contains("error: no common ancestor found"));
    }

    #[test]
    fn union_strategy_runs_lanes_in_ours_base_theirs_order() {
        let output = run(
            "<<<<<<< print\nours\n||||||| base\nbase\n=======\ntheirs\n>>>>>>> print\n",
            Strategy::Union,
        );

        assert_eq!(output.exit_code, 0);
        assert_eq!(output.stdout, "ours\nbase\ntheirs\n");
    }

    #[test]
    fn seeded_random_strategy_is_reproducible() {
        let source = "<<<<<<< print\nours\n||||||| base\nbase\n=======\ntheirs\n>>>>>>> print\n";
        let left = run_seeded(source, Strategy::Random, 42);
        let right = run_seeded(source, Strategy::Random, 42);

        assert_eq!(left.exit_code, 0);
        assert_eq!(left.stdout, right.stdout);
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
    fn patch_mode_executes_added_and_context_lines() {
        let output = run(
            "diff --git a/in b/out\n@@ -1,2 +1,2 @@\n<<<<<<< print\n+added\n-removed\n context\n=======\n+fallback\n>>>>>>> print\n",
            Strategy::Ours,
        );

        assert_eq!(output.exit_code, 0);
        assert_eq!(output.stdout, "added\ncontext\n");
    }

    #[test]
    fn plus_and_minus_lines_are_preserved_outside_patch_mode() {
        let output = run(
            "<<<<<<< print\n+added\n-removed\n=======\nother\n>>>>>>> print\n",
            Strategy::Ours,
        );

        assert_eq!(output.exit_code, 0);
        assert_eq!(output.stdout, "+added\n-removed\n");
    }

    #[test]
    fn status_mode_populates_runtime_metadata() {
        let output = run_with_parse_options(
            "On branch main\n<<<<<<< print\n${git.branch}\n=======\nno\n>>>>>>> print\n",
            ParseOptions {
                accept_regret: false,
                git_status_mode: true,
            },
        );

        assert_eq!(output.exit_code, 0);
        assert_eq!(output.stdout, "main\n");
    }

    #[test]
    fn status_lines_are_raw_without_status_mode() {
        let output = run(
            "On branch main\n<<<<<<< print\n${git.branch}\n=======\nno\n>>>>>>> print\n",
            Strategy::Ours,
        );

        assert_eq!(output.exit_code, 0);
        assert_eq!(output.stdout, "\n");
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
    fn let_binds_selected_value_for_interpolation() {
        let output = run(
            "<<<<<<< let name\nJames\n=======\nUser\n>>>>>>> let name\n<<<<<<< print\nHello, ${name}\n=======\nGoodbye, ${name}\n>>>>>>> print\n",
            Strategy::Ours,
        );

        assert_eq!(output.exit_code, 0);
        assert_eq!(output.stdout, "Hello, James\n");
    }

    #[test]
    fn let_uses_theirs_value_under_theirs_strategy() {
        let output = run(
            "<<<<<<< let name\nJames\n=======\nUser\n>>>>>>> let name\n<<<<<<< print\nHello, ${name}\n=======\nGoodbye, ${name}\n>>>>>>> print\n",
            Strategy::Theirs,
        );

        assert_eq!(output.exit_code, 0);
        assert_eq!(output.stdout, "Goodbye, User\n");
    }

    #[test]
    fn let_checks_base_lane_type_metadata() {
        let output = run(
            "<<<<<<< let age\n30\n||||||| int default\n0\n=======\nthirty\n>>>>>>> let age\n<<<<<<< print\n${age}\n=======\n${age}\n>>>>>>> print\n",
            Strategy::Ours,
        );

        assert_eq!(output.exit_code, 0);
        assert_eq!(output.stdout, "30\n");
    }

    #[test]
    fn let_type_mismatch_renders_type_conflict() {
        let output = run(
            "<<<<<<< let age\nthirty\n||||||| int default\n0\n=======\n30\n>>>>>>> let age\n",
            Strategy::Ours,
        );

        assert_eq!(output.exit_code, 1);
        assert!(output
            .stderr
            .contains("CONFLICT (type): Merge conflict in age"));
        assert!(output
            .stderr
            .contains("<<<<<<< expected\nint\n=======\nstring\n"));
    }

    #[test]
    fn missing_interpolation_value_becomes_empty_text() {
        let output = run(
            "<<<<<<< print\nHello, ${missing}\n=======\nGoodbye\n>>>>>>> print\n",
            Strategy::Ours,
        );

        assert_eq!(output.exit_code, 0);
        assert_eq!(output.stdout, "Hello, \n");
    }

    #[test]
    fn let_without_name_errors() {
        let output = run(
            "<<<<<<< let\nvalue\n=======\nother\n>>>>>>> let\n",
            Strategy::Ours,
        );

        assert_eq!(output.exit_code, 1);
        assert!(output.stderr.contains("let requires a binding name"));
    }

    #[test]
    fn if_runs_ours_when_condition_is_truthy() {
        let output = run(
            "<<<<<<< let enabled\ntrue\n=======\nfalse\n>>>>>>> let enabled\n<<<<<<< if enabled\n<<<<<<< print\nyes\n=======\nno\n>>>>>>> print\n=======\n<<<<<<< print\nfallback\n=======\nnope\n>>>>>>> print\n>>>>>>> if\n",
            Strategy::Ours,
        );

        assert_eq!(output.exit_code, 0);
        assert_eq!(output.stdout, "yes\n");
    }

    #[test]
    fn if_runs_theirs_when_condition_is_false() {
        let output = run(
            "<<<<<<< if false\n<<<<<<< print\nyes\n=======\nno\n>>>>>>> print\n=======\n<<<<<<< print\nfallback\n=======\nnope\n>>>>>>> print\n>>>>>>> if\n",
            Strategy::Ours,
        );

        assert_eq!(output.exit_code, 0);
        assert_eq!(output.stdout, "fallback\n");
    }

    #[test]
    fn repeat_runs_selected_body_count_times() {
        let output = run(
            "<<<<<<< repeat 3\n<<<<<<< print\nagain\n=======\nno\n>>>>>>> print\n=======\ndone\n>>>>>>> repeat\n",
            Strategy::Ours,
        );

        assert_eq!(output.exit_code, 0);
        assert_eq!(output.stdout, "again\nagain\nagain\n");
    }

    #[test]
    fn repeat_rejects_bad_count() {
        let output = run(
            "<<<<<<< repeat many\ntext\n=======\nother\n>>>>>>> repeat\n",
            Strategy::Ours,
        );

        assert_eq!(output.exit_code, 1);
        assert!(output
            .stderr
            .contains("repeat count must be a non-negative integer: many"));
    }

    #[test]
    fn function_and_call_bind_arguments_in_call_scope() {
        let output = run(
            "<<<<<<< function greet person\n<<<<<<< print\nHello, ${person}\n=======\nBye, ${person}\n>>>>>>> print\n=======\nignored\n>>>>>>> function greet\n<<<<<<< call greet\nJames\n=======\nNobody\n>>>>>>> call greet\n",
            Strategy::Ours,
        );

        assert_eq!(output.exit_code, 0);
        assert_eq!(output.stdout, "Hello, James\n");
    }

    #[test]
    fn function_call_under_theirs_uses_theirs_body_and_argument() {
        let output = run(
            "<<<<<<< function greet person\n<<<<<<< print\nHello, ${person}\n=======\nBye, ${person}\n>>>>>>> print\n=======\nignored\n>>>>>>> function greet\n<<<<<<< call greet\nJames\n=======\nNobody\n>>>>>>> call greet\n",
            Strategy::Theirs,
        );

        assert_eq!(output.exit_code, 0);
        assert_eq!(output.stdout, "");
    }

    #[test]
    fn call_unknown_function_errors() {
        let output = run(
            "<<<<<<< call missing\nJames\n=======\nNobody\n>>>>>>> call missing\n",
            Strategy::Ours,
        );

        assert_eq!(output.exit_code, 1);
        assert!(output.stderr.contains("unknown function `missing`"));
    }

    #[test]
    fn return_stops_function_body() {
        let output = run(
            "<<<<<<< function stop\n<<<<<<< return\ndone\n=======\nno\n>>>>>>> return\n<<<<<<< print\nafter\n=======\nafter\n>>>>>>> print\n=======\nignored\n>>>>>>> function stop\n<<<<<<< call stop\n=======\n>>>>>>> call stop\n<<<<<<< print\noutside\n=======\nno\n>>>>>>> print\n",
            Strategy::Ours,
        );

        assert_eq!(output.exit_code, 0);
        assert_eq!(output.stdout, "outside\n");
    }

    #[test]
    fn throw_returns_runtime_conflict() {
        let output = run(
            "<<<<<<< throw\nsomething exploded\n=======\nquiet\n>>>>>>> throw\n",
            Strategy::Ours,
        );

        assert_eq!(output.exit_code, 1);
        assert!(output
            .stderr
            .contains("CONFLICT (runtime): Merge conflict in execution"));
        assert!(output.stderr.contains("something exploded"));
    }

    #[test]
    fn try_runs_attempt_when_it_succeeds() {
        let output = run(
            "<<<<<<< try\n<<<<<<< print\nattempt\n=======\nno\n>>>>>>> print\n=======\n<<<<<<< print\nrecovery\n=======\nno\n>>>>>>> print\n>>>>>>> try\n",
            Strategy::Ours,
        );

        assert_eq!(output.exit_code, 0);
        assert_eq!(output.stdout, "attempt\n");
    }

    #[test]
    fn try_runs_recovery_when_attempt_fails() {
        let output = run(
            "<<<<<<< try\n<<<<<<< throw\nboom\n=======\nno\n>>>>>>> throw\n=======\n<<<<<<< print\nrecovered\n=======\nno\n>>>>>>> print\n>>>>>>> try\n",
            Strategy::Ours,
        );

        assert_eq!(output.exit_code, 0);
        assert_eq!(output.stdout, "recovered\n");
    }

    #[test]
    fn try_runs_base_cleanup_after_attempt() {
        let output = run(
            "<<<<<<< try\n<<<<<<< print\nattempt\n=======\nno\n>>>>>>> print\n||||||| finally\n<<<<<<< print\ncleanup\n=======\nno\n>>>>>>> print\n=======\n<<<<<<< print\nrecovery\n=======\nno\n>>>>>>> print\n>>>>>>> try\n",
            Strategy::Ours,
        );

        assert_eq!(output.exit_code, 0);
        assert_eq!(output.stdout, "attempt\ncleanup\n");
    }

    #[test]
    fn resolve_overrides_nested_strategy() {
        let output = run(
            "<<<<<<< resolve theirs\n<<<<<<< print\nours\n=======\ntheirs\n>>>>>>> print\n=======\nignored\n>>>>>>> resolve\n",
            Strategy::Ours,
        );

        assert_eq!(output.exit_code, 0);
        assert_eq!(output.stdout, "theirs\n");
    }

    #[test]
    fn resolve_requires_strategy() {
        let output = run(
            "<<<<<<< resolve\nbody\n=======\nother\n>>>>>>> resolve\n",
            Strategy::Ours,
        );

        assert_eq!(output.exit_code, 1);
        assert!(output.stderr.contains("resolve requires a strategy"));
    }

    #[test]
    fn import_evaluates_local_file_relative_to_source() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("mergehell_import_{unique}"));
        fs::create_dir_all(&dir).unwrap();
        let imported = dir.join("lib.mh");
        fs::write(
            &imported,
            "<<<<<<< print\nfrom import\n=======\nno\n>>>>>>> print\n",
        )
        .unwrap();

        let main = "<<<<<<< import\nlib.mh\n=======\nmissing.mh\n>>>>>>> import\n";
        let output = run_named(dir.join("main.mh").to_str().unwrap(), main, Strategy::Ours);

        assert_eq!(output.exit_code, 0);
        assert_eq!(output.stdout, "from import\n");
    }

    #[test]
    fn import_missing_file_errors() {
        let output = run(
            "<<<<<<< import\nmissing.mh\n=======\nother.mh\n>>>>>>> import\n",
            Strategy::Ours,
        );

        assert_eq!(output.exit_code, 1);
        assert!(output.stderr.contains("could not import missing.mh"));
    }

    #[test]
    fn import_standard_module_succeeds_without_file_io() {
        let output = run(
            "<<<<<<< import\nrerere\n=======\nmissing.mh\n>>>>>>> import\n<<<<<<< print\n${stdlib.rerere.loaded}\n=======\nno\n>>>>>>> print\n",
            Strategy::Ours,
        );

        assert_eq!(output.exit_code, 0);
        assert_eq!(output.stdout, "true\n");
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
