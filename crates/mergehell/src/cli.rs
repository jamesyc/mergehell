use std::fs;
use std::io::{self, Read};
use std::process::ExitCode;

use crate::diagnostic::render_diagnostics;
use crate::resolve::strategy::Strategy;

pub fn main() -> ExitCode {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let result = run_cli(&args, &mut io::stdin());
    print!("{}", result.stdout);
    eprint!("{}", result.stderr);
    ExitCode::from(result.exit_code as u8)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CliOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

pub fn run_cli(args: &[String], stdin: &mut dyn Read) -> CliOutput {
    if args.is_empty() || args[0] == "--help" || args[0] == "-h" {
        return CliOutput::success(help_text());
    }

    match args[0].as_str() {
        "run" => run_command(&args[1..], stdin),
        "check" => check_command(&args[1..], stdin),
        "ast" => ast_command(&args[1..], stdin),
        "format" => format_command(&args[1..], stdin),
        "merge" | "regret" => CliOutput::failure(format!(
            "error: `{}` is declared but not implemented in Level 0\n",
            args[0]
        )),
        command => CliOutput::usage_error(format!("error: unknown command `{command}`\n")),
    }
}

fn run_command(args: &[String], stdin: &mut dyn Read) -> CliOutput {
    if args.is_empty() {
        return CliOutput::usage_error("error: run requires FILE\n".to_string());
    }

    let file = &args[0];
    let mut strategy = Strategy::Ours;
    for arg in &args[1..] {
        match arg.parse::<Strategy>() {
            Ok(parsed) => strategy = parsed,
            Err(message) => return CliOutput::usage_error(format!("error: {message}\n")),
        }
    }

    let source = match read_source(file, stdin) {
        Ok(source) => source,
        Err(error) => return CliOutput::failure(format!("error: {error}\n")),
    };
    let output = crate::run(file, source, strategy);
    CliOutput {
        stdout: output.stdout,
        stderr: output.stderr,
        exit_code: output.exit_code,
    }
}

fn check_command(args: &[String], stdin: &mut dyn Read) -> CliOutput {
    if args.len() != 1 {
        return CliOutput::usage_error("error: check requires FILE\n".to_string());
    }

    let file = &args[0];
    let source = match read_source(file, stdin) {
        Ok(source) => source,
        Err(error) => return CliOutput::failure(format!("error: {error}\n")),
    };

    match crate::check(file, source) {
        Ok(()) => CliOutput::success(String::new()),
        Err(diagnostics) => CliOutput {
            stdout: String::new(),
            stderr: render_diagnostics(&diagnostics),
            exit_code: 1,
        },
    }
}

fn ast_command(args: &[String], stdin: &mut dyn Read) -> CliOutput {
    if args.len() != 1 {
        return CliOutput::usage_error("error: ast requires FILE\n".to_string());
    }

    let file = &args[0];
    let source = match read_source(file, stdin) {
        Ok(source) => source,
        Err(error) => return CliOutput::failure(format!("error: {error}\n")),
    };
    CliOutput::success(format!("{}\n", crate::ast(file, source)))
}

fn format_command(args: &[String], stdin: &mut dyn Read) -> CliOutput {
    if args.len() != 1 {
        return CliOutput::usage_error("error: format requires FILE\n".to_string());
    }

    let file = &args[0];
    let source = match read_source(file, stdin) {
        Ok(source) => source,
        Err(error) => return CliOutput::failure(format!("error: {error}\n")),
    };
    CliOutput::success(crate::format_source(file, source))
}

fn read_source(path: &str, stdin: &mut dyn Read) -> io::Result<String> {
    if path == "-" {
        let mut source = String::new();
        stdin.read_to_string(&mut source)?;
        Ok(source)
    } else {
        fs::read_to_string(path)
    }
}

fn help_text() -> String {
    "MergeHell reference interpreter\n\nUSAGE:\n    mergehell <COMMAND> [ARGS]\n\nCOMMANDS:\n    mergehell run FILE [--ours|--theirs]\n    mergehell check FILE\n    mergehell ast FILE\n    mergehell format FILE\n    mergehell merge BASE OURS THEIRS\n    mergehell regret FILE\n\n".to_string()
}

impl CliOutput {
    fn success(stdout: String) -> Self {
        Self {
            stdout,
            stderr: String::new(),
            exit_code: 0,
        }
    }

    fn failure(stderr: String) -> Self {
        Self {
            stdout: String::new(),
            stderr,
            exit_code: 1,
        }
    }

    fn usage_error(stderr: String) -> Self {
        Self {
            stdout: String::new(),
            stderr,
            exit_code: 2,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn help_succeeds() {
        let output = run_cli(&args(&["--help"]), &mut Cursor::new(""));

        assert_eq!(output.exit_code, 0);
        assert!(output.stdout.contains("mergehell run FILE"));
    }

    #[test]
    fn unknown_command_is_usage_error() {
        let output = run_cli(&args(&["explode"]), &mut Cursor::new(""));

        assert_eq!(output.exit_code, 2);
        assert_eq!(output.stderr, "error: unknown command `explode`\n");
    }

    #[test]
    fn run_reads_stdin_and_uses_ours_by_default() {
        let source = "<<<<<<< print\nHello\n=======\nGoodbye\n>>>>>>> print\n";
        let output = run_cli(&args(&["run", "-"]), &mut Cursor::new(source));

        assert_eq!(output.exit_code, 0);
        assert_eq!(output.stdout, "Hello\n");
    }

    #[test]
    fn run_accepts_theirs_strategy() {
        let source = "<<<<<<< print\nHello\n=======\nGoodbye\n>>>>>>> print\n";
        let output = run_cli(&args(&["run", "-", "--theirs"]), &mut Cursor::new(source));

        assert_eq!(output.exit_code, 0);
        assert_eq!(output.stdout, "Goodbye\n");
    }

    #[test]
    fn run_rejects_unsupported_strategy() {
        let source = "<<<<<<< print\nHello\n=======\nGoodbye\n>>>>>>> print\n";
        let output = run_cli(&args(&["run", "-", "--union"]), &mut Cursor::new(source));

        assert_eq!(output.exit_code, 2);
        assert_eq!(output.stderr, "error: unsupported strategy: --union\n");
    }

    #[test]
    fn check_reads_stdin_and_fails_clean_source() {
        let output = run_cli(&args(&["check", "-"]), &mut Cursor::new("clean\n"));

        assert_eq!(output.exit_code, 1);
        assert!(output.stderr.contains("fatal: no conflict markers found"));
    }

    #[test]
    fn check_succeeds_for_conflict_source() {
        let source = "<<<<<<< print\nHello\n=======\nGoodbye\n>>>>>>> print\n";
        let output = run_cli(&args(&["check", "-"]), &mut Cursor::new(source));

        assert_eq!(output.exit_code, 0);
        assert_eq!(output.stderr, "");
    }

    #[test]
    fn ast_reads_stdin() {
        let source = "<<<<<<< print\nHello\n=======\nGoodbye\n>>>>>>> print\n";
        let output = run_cli(&args(&["ast", "-"]), &mut Cursor::new(source));

        assert_eq!(output.exit_code, 0);
        assert!(output.stdout.contains("ConflictNode"));
    }

    #[test]
    fn placeholder_commands_fail_clearly() {
        let output = run_cli(&args(&["regret", "-"]), &mut Cursor::new(""));

        assert_eq!(output.exit_code, 1);
        assert_eq!(
            output.stderr,
            "error: `regret` is declared but not implemented in Level 0\n"
        );
    }
}
