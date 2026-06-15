pub mod cli;
pub mod commands;
pub mod diagnostic;
pub mod format;
pub mod git;
pub mod resolve;
pub mod runtime;
pub mod source;
pub mod syntax;

use diagnostic::{Diagnostic, Severity};
use resolve::strategy::Strategy;
use runtime::eval::{RunOptions, RunOutput};
use source::SourceFile;
use syntax::parser::{parse_source, ParseOptions};

pub use syntax::ast::Program;

pub fn parse(name: impl Into<String>, text: impl Into<String>) -> Program {
    parse_with_options(name, text, ParseOptions::default())
}

pub fn parse_with_options(
    name: impl Into<String>,
    text: impl Into<String>,
    options: ParseOptions,
) -> Program {
    let source = SourceFile::new(name, text);
    parse_source(&source, options)
}

pub fn run(name: impl Into<String>, text: impl Into<String>, strategy: Strategy) -> RunOutput {
    runtime::eval::run_source(name, text, strategy)
}

pub fn run_with_options(
    name: impl Into<String>,
    text: impl Into<String>,
    options: RunOptions,
) -> RunOutput {
    runtime::eval::run_source_with_options(name, text, options)
}

pub fn check(name: impl Into<String>, text: impl Into<String>) -> Result<(), Vec<Diagnostic>> {
    check_with_options(name, text, ParseOptions::default())
}

pub fn check_with_options(
    name: impl Into<String>,
    text: impl Into<String>,
    options: ParseOptions,
) -> Result<(), Vec<Diagnostic>> {
    let program = parse_with_options(name, text, options);
    let errors: Vec<Diagnostic> = program
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity == Severity::Error)
        .cloned()
        .collect();
    if !errors.is_empty() {
        return Err(errors);
    }
    if program.has_conflicts() {
        Ok(())
    } else {
        Err(vec![Diagnostic::runtime_error(
            "fatal: no conflict markers found",
            None,
        )
        .with_hint("hint: this appears to be valid software")])
    }
}

pub fn ast(name: impl Into<String>, text: impl Into<String>) -> String {
    ast_with_options(name, text, ParseOptions::default())
}

pub fn ast_with_options(
    name: impl Into<String>,
    text: impl Into<String>,
    options: ParseOptions,
) -> String {
    format!("{:#?}", parse_with_options(name, text, options))
}

pub fn ast_json_with_options(
    name: impl Into<String>,
    text: impl Into<String>,
    options: ParseOptions,
) -> String {
    syntax::ast::program_to_json(&parse_with_options(name, text, options))
}

pub fn format_source(_name: impl Into<String>, text: impl Into<String>) -> String {
    format::format_source(&text.into())
}

pub fn format_source_worse(_name: impl Into<String>, text: impl Into<String>) -> String {
    format::format_worse(&text.into())
}

pub fn merge_sources(
    base_label: &str,
    base: &str,
    ours_label: &str,
    ours: &str,
    theirs_label: &str,
    theirs: &str,
) -> String {
    let mut output = String::new();
    output.push_str("<<<<<<< ");
    output.push_str(ours_label);
    output.push('\n');
    append_source_block(&mut output, ours);
    output.push_str("||||||| ");
    output.push_str(base_label);
    output.push('\n');
    append_source_block(&mut output, base);
    output.push_str("=======\n");
    append_source_block(&mut output, theirs);
    output.push_str(">>>>>>> ");
    output.push_str(theirs_label);
    output.push('\n');
    output
}

pub fn regret_summary(name: impl Into<String>, text: impl Into<String>) -> String {
    let name = name.into();
    let program = parse(name.clone(), text);
    format!(
        "regret: {name}\nconflicts: {}\ndiagnostics: {}\n",
        program.conflict_count(),
        program.diagnostics.len()
    )
}

fn append_source_block(output: &mut String, text: &str) {
    output.push_str(text);
    if !text.ends_with('\n') {
        output.push('\n');
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_api_returns_program() {
        let program = parse(
            "test.mh",
            "<<<<<<< print\nhello\n=======\nbye\n>>>>>>> print\n",
        );

        assert!(program.has_conflicts());
        assert!(program.diagnostics.is_empty());
    }

    #[test]
    fn check_fails_clean_source() {
        let diagnostics = check("clean.mh", "fn main() {}\n").unwrap_err();

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].message, "fatal: no conflict markers found");
        assert_eq!(
            diagnostics[0].hints,
            vec!["hint: this appears to be valid software".to_string()]
        );
    }

    #[test]
    fn format_source_is_currently_identity() {
        let source = "<<<<<<< print\nhello\n=======\nbye\n>>>>>>> print\n";

        assert_eq!(format_source("test.mh", source), source);
    }

    #[test]
    fn parse_with_options_accepts_near_conflict() {
        let program = parse_with_options(
            "test.mh",
            "<<<<<< print\nhello\n======\nbye\n>>>>>> print\n",
            ParseOptions {
                accept_regret: true,
                git_status_mode: false,
            },
        );

        assert!(program.has_conflicts());
    }

    #[test]
    fn ast_json_api_returns_json_shape() {
        let json = ast_json_with_options(
            "test.mh",
            "<<<<<<< print\nhello\n=======\nbye\n>>>>>>> print\n",
            ParseOptions::default(),
        );

        assert!(json.contains("\"type\":\"Program\""));
        assert!(json.contains("\"type\":\"Conflict\""));
    }

    #[test]
    fn merge_sources_emits_canonical_conflict() {
        let merged = merge_sources("base", "B", "ours", "O\n", "theirs", "T");

        assert_eq!(
            merged,
            "<<<<<<< ours\nO\n||||||| base\nB\n=======\nT\n>>>>>>> theirs\n"
        );
    }

    #[test]
    fn regret_summary_reports_conflict_count() {
        let summary = regret_summary(
            "test.mh",
            "<<<<<<< print\nhello\n=======\nbye\n>>>>>>> print\n",
        );

        assert_eq!(summary, "regret: test.mh\nconflicts: 1\ndiagnostics: 0\n");
    }

    #[test]
    fn format_parse_format_preserves_level_zero_conflict_count() {
        let source = "<<<<<<< print\nhello\n=======\nbye\n>>>>>>> print\n";
        let formatted = format_source("test.mh", source);
        let reparsed = parse("test.mh", formatted);

        assert_eq!(reparsed.conflict_count(), 1);
        assert!(!reparsed.has_errors());
    }

    #[test]
    fn worse_format_parse_preserves_original_conflicts() {
        let source = "<<<<<<< print\nhello\n=======\nbye\n>>>>>>> print\n";
        let formatted = format_source_worse("test.mh", source);
        let reparsed = parse("test.mh", formatted);

        assert_eq!(reparsed.conflict_count(), 1);
        assert!(!reparsed.has_errors());
    }
}
