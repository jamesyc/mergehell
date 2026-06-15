use std::fs;
use std::io::{self, Read};
use std::process::ExitCode;

use crate::diagnostic::render_diagnostics;
use crate::resolve::strategy::Strategy;
use crate::runtime::eval::RunOptions;
use crate::syntax::parser::ParseOptions;

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
        "merge" => merge_command(&args[1..], stdin),
        "regret" => regret_command(&args[1..], stdin),
        command => CliOutput::usage_error(format!("error: unknown command `{command}`\n")),
    }
}

fn run_command(args: &[String], stdin: &mut dyn Read) -> CliOutput {
    if args.is_empty() {
        return CliOutput::usage_error("error: run requires FILE\n".to_string());
    }

    let file = &args[0];
    let options = match parse_cli_options(&args[1..]) {
        Ok(options) => options,
        Err(message) => return CliOutput::usage_error(format!("error: {message}\n")),
    };

    let source = match read_source(file, stdin) {
        Ok(source) => source,
        Err(error) => return CliOutput::failure(format!("error: {error}\n")),
    };
    let output = crate::run_with_options(file, source, options.into_run_options());
    CliOutput {
        stdout: output.stdout,
        stderr: output.stderr,
        exit_code: output.exit_code,
    }
}

fn check_command(args: &[String], stdin: &mut dyn Read) -> CliOutput {
    if args.is_empty() {
        return CliOutput::usage_error("error: check requires FILE\n".to_string());
    }

    let file = &args[0];
    let options = match parse_cli_options(&args[1..]) {
        Ok(options) => options,
        Err(message) => return CliOutput::usage_error(format!("error: {message}\n")),
    };
    let source = match read_source(file, stdin) {
        Ok(source) => source,
        Err(error) => return CliOutput::failure(format!("error: {error}\n")),
    };

    let check_result = if options.strict {
        strict_check(file, source, options.parse_options)
    } else {
        crate::check_with_options(file, source, options.parse_options)
    };

    match check_result {
        Ok(()) => CliOutput::success(String::new()),
        Err(diagnostics) => CliOutput {
            stdout: String::new(),
            stderr: render_diagnostics(&diagnostics),
            exit_code: 1,
        },
    }
}

fn ast_command(args: &[String], stdin: &mut dyn Read) -> CliOutput {
    if args.is_empty() {
        return CliOutput::usage_error("error: ast requires FILE\n".to_string());
    }

    let file = &args[0];
    let (json, option_args) = split_ast_args(&args[1..]);
    let options = match parse_cli_options(&option_args) {
        Ok(options) => options,
        Err(message) => return CliOutput::usage_error(format!("error: {message}\n")),
    };
    let source = match read_source(file, stdin) {
        Ok(source) => source,
        Err(error) => return CliOutput::failure(format!("error: {error}\n")),
    };
    if json {
        CliOutput::success(format!(
            "{}\n",
            crate::ast_json_with_options(file, source, options.parse_options)
        ))
    } else {
        CliOutput::success(format!(
            "{}\n",
            crate::ast_with_options(file, source, options.parse_options)
        ))
    }
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

fn merge_command(args: &[String], stdin: &mut dyn Read) -> CliOutput {
    if args.len() != 3 {
        return CliOutput::usage_error("error: merge requires BASE OURS THEIRS\n".to_string());
    }

    let base = match read_source(&args[0], stdin) {
        Ok(source) => source,
        Err(error) => return CliOutput::failure(format!("error: {error}\n")),
    };
    let ours = match read_source(&args[1], stdin) {
        Ok(source) => source,
        Err(error) => return CliOutput::failure(format!("error: {error}\n")),
    };
    let theirs = match read_source(&args[2], stdin) {
        Ok(source) => source,
        Err(error) => return CliOutput::failure(format!("error: {error}\n")),
    };

    CliOutput::success(crate::merge_sources(
        &args[0], &base, &args[1], &ours, &args[2], &theirs,
    ))
}

fn regret_command(args: &[String], stdin: &mut dyn Read) -> CliOutput {
    if args.is_empty() {
        return CliOutput::usage_error("error: regret requires FILE\n".to_string());
    }

    let file = &args[0];
    let source = match read_source(file, stdin) {
        Ok(source) => source,
        Err(error) => return CliOutput::failure(format!("error: {error}\n")),
    };

    CliOutput::success(crate::regret_summary(file, source))
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct CliOptions {
    strategy: Strategy,
    seed: u64,
    parse_options: ParseOptions,
    strict: bool,
}

impl Default for CliOptions {
    fn default() -> Self {
        Self {
            strategy: Strategy::Ours,
            seed: 0,
            parse_options: ParseOptions::default(),
            strict: false,
        }
    }
}

impl CliOptions {
    fn into_run_options(self) -> RunOptions {
        RunOptions {
            strategy: self.strategy,
            seed: self.seed,
            parse_options: self.parse_options,
            strict: self.strict,
        }
    }
}

fn parse_cli_options(args: &[String]) -> Result<CliOptions, String> {
    let mut options = CliOptions::default();
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--seed" => {
                let Some(raw_seed) = args.get(index + 1) else {
                    return Err("--seed requires a value".to_string());
                };
                options.seed = raw_seed
                    .parse::<u64>()
                    .map_err(|_| format!("--seed must be an unsigned integer: {raw_seed}"))?;
                index += 2;
            }
            "--accept-regret" => {
                options.parse_options.accept_regret = true;
                index += 1;
            }
            "--git-status-mode" => {
                options.parse_options.git_status_mode = true;
                index += 1;
            }
            "--strict" => {
                options.strict = true;
                index += 1;
            }
            flag => {
                options.strategy = flag.parse::<Strategy>()?;
                index += 1;
            }
        }
    }

    Ok(options)
}

fn split_ast_args(args: &[String]) -> (bool, Vec<String>) {
    let mut json = false;
    let mut option_args = Vec::new();
    for arg in args {
        if arg == "--json" {
            json = true;
        } else {
            option_args.push(arg.clone());
        }
    }
    (json, option_args)
}

fn strict_check(
    name: impl Into<String>,
    text: impl Into<String>,
    parse_options: ParseOptions,
) -> Result<(), Vec<crate::diagnostic::Diagnostic>> {
    let program = crate::parse_with_options(name, text, parse_options);
    if !program.diagnostics.is_empty() {
        Err(program.diagnostics)
    } else if program.has_conflicts() {
        Ok(())
    } else {
        crate::check("__clean__", "")
    }
}

fn help_text() -> String {
    "MergeHell reference interpreter\n\nUSAGE:\n    mergehell <COMMAND> [ARGS]\n\nCOMMANDS:\n    mergehell run FILE [--ours|--theirs|--base|--union|--random|--git] [--seed N] [--accept-regret] [--strict]\n    mergehell check FILE [--accept-regret] [--strict]\n    mergehell ast FILE [--json] [--accept-regret]\n    mergehell format FILE\n    mergehell merge BASE OURS THEIRS\n    mergehell regret FILE\n\n".to_string()
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
    use std::fs;
    use std::io::Cursor;
    use std::time::{SystemTime, UNIX_EPOCH};

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
        let output = run_cli(&args(&["run", "-", "--manual"]), &mut Cursor::new(source));

        assert_eq!(output.exit_code, 2);
        assert_eq!(output.stderr, "error: unsupported strategy: --manual\n");
    }

    #[test]
    fn run_accepts_union_strategy() {
        let source = "<<<<<<< print\nHello\n=======\nGoodbye\n>>>>>>> print\n";
        let output = run_cli(&args(&["run", "-", "--union"]), &mut Cursor::new(source));

        assert_eq!(output.exit_code, 0);
        assert_eq!(output.stdout, "Hello\nGoodbye\n");
    }

    #[test]
    fn run_accepts_seeded_random_strategy() {
        let source = "<<<<<<< print\nHello\n=======\nGoodbye\n>>>>>>> print\n";
        let left = run_cli(
            &args(&["run", "-", "--random", "--seed", "7"]),
            &mut Cursor::new(source),
        );
        let right = run_cli(
            &args(&["run", "-", "--random", "--seed", "7"]),
            &mut Cursor::new(source),
        );

        assert_eq!(left.exit_code, 0);
        assert_eq!(left.stdout, right.stdout);
    }

    #[test]
    fn seed_requires_numeric_value() {
        let output = run_cli(
            &args(&["run", "-", "--random", "--seed", "bad"]),
            &mut Cursor::new(""),
        );

        assert_eq!(output.exit_code, 2);
        assert_eq!(
            output.stderr,
            "error: --seed must be an unsigned integer: bad\n"
        );
    }

    #[test]
    fn accept_regret_allows_near_conflict() {
        let source = "<<<<<< print\nHello\n======\nGoodbye\n>>>>>> print\n";
        let output = run_cli(
            &args(&["run", "-", "--accept-regret"]),
            &mut Cursor::new(source),
        );

        assert_eq!(output.exit_code, 0);
        assert_eq!(output.stdout, "Hello\n");
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
    fn ast_json_reads_stdin() {
        let source = "<<<<<<< print\nHello\n=======\nGoodbye\n>>>>>>> print\n";
        let output = run_cli(&args(&["ast", "-", "--json"]), &mut Cursor::new(source));

        assert_eq!(output.exit_code, 0);
        assert!(output.stdout.contains("\"type\":\"Program\""));
        assert!(output.stdout.contains("\"type\":\"Conflict\""));
    }

    #[test]
    fn strict_run_fails_on_parser_warning() {
        let source = "  <<<<<<< print\nHello\n=======\nGoodbye\n>>>>>>> print\n";
        let output = run_cli(&args(&["run", "-", "--strict"]), &mut Cursor::new(source));

        assert_eq!(output.exit_code, 1);
        assert!(output
            .stderr
            .contains("warning: indented conflict marker detected"));
    }

    #[test]
    fn regret_summarizes_conflicts() {
        let source = "<<<<<<< print\nHello\n=======\nGoodbye\n>>>>>>> print\n";
        let output = run_cli(&args(&["regret", "-"]), &mut Cursor::new(source));

        assert_eq!(output.exit_code, 0);
        assert_eq!(output.stdout, "regret: -\nconflicts: 1\ndiagnostics: 0\n");
    }

    #[test]
    fn merge_emits_canonical_conflict() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("mergehell_merge_{unique}"));
        fs::create_dir_all(&dir).unwrap();
        let base = dir.join("base.mh");
        let ours = dir.join("ours.mh");
        let theirs = dir.join("theirs.mh");
        fs::write(&base, "base\n").unwrap();
        fs::write(&ours, "ours\n").unwrap();
        fs::write(&theirs, "theirs\n").unwrap();

        let output = run_cli(
            &args(&[
                "merge",
                base.to_str().unwrap(),
                ours.to_str().unwrap(),
                theirs.to_str().unwrap(),
            ]),
            &mut Cursor::new(""),
        );

        assert_eq!(output.exit_code, 0);
        assert!(output.stdout.contains("<<<<<<< "));
        assert!(output.stdout.contains("ours\n"));
        assert!(output.stdout.contains("||||||| "));
        assert!(output.stdout.contains("base\n"));
        assert!(output.stdout.contains("theirs\n"));
    }
}
