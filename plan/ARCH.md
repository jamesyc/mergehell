# MergeHell Rust Architecture

This document describes the planned Rust architecture for implementing the
MergeHell language from `SPEC.md`.

The implementation should make unresolved Git conflict syntax a first-class
program representation while keeping parsing, resolution, execution, and CLI
concerns separated. The main design goal is a small, testable interpreter that
can reach compliance Level 0 quickly and then grow toward the richer Git-aware
features without rewriting the core.

## Design Goals

- Preserve source text, marker labels, metadata, and source locations.
- Parse nested conflict hunks with a stack-based, line-oriented parser.
- Keep resolution strategy separate from command execution.
- Make diagnostics structured internally and render them as valid MergeHell
  source at the boundary.
- Make required strategies and commands pluggable enough for optional Git,
  blame, rerere, and formatter features later.
- Favor deterministic behavior in tests, especially for `random`.
- Keep the initial runtime synchronous and single-threaded unless a later
  feature proves it needs concurrency.

## Proposed Repository Layout

Start with one Cargo workspace and one primary library crate. This keeps
boundaries explicit without forcing premature crate splits.

```txt
Cargo.toml
crates/
  mergehell/
    Cargo.toml
    src/
      lib.rs
      bin/
        mergehell.rs
      cli.rs
      diagnostic.rs
      source.rs
      syntax/
        mod.rs
        ast.rs
        line.rs
        parser.rs
      resolve/
        mod.rs
        strategy.rs
        rng.rs
      runtime/
        mod.rs
        context.rs
        eval.rs
        value.rs
        control.rs
      commands/
        mod.rs
        print.rs
        binding.rs
        control_flow.rs
        functions.rs
        import.rs
      git/
        mod.rs
        diff.rs
        status.rs
        merge_driver.rs
      format/
        mod.rs
tests/
  fixtures/
```

If the codebase grows, the natural crate split is:

- `mergehell-syntax`: AST, parser, source spans, textual diagnostics.
- `mergehell-runtime`: resolver, interpreter, command registry, values.
- `mergehell-cli`: command-line frontend.

The first implementation should not split crates until API boundaries are
stable.

## Execution Pipeline

```txt
Input bytes
  -> SourceFile
  -> line classification
  -> parser
  -> Program AST
  -> strategy resolver during evaluation
  -> command execution
  -> stdout, stderr diagnostics, exit code
```

The resolver should run as part of evaluation instead of pre-flattening the
whole AST. Lazy resolution is simpler for nested conflicts, `if`, `try`,
`resolve`, imports, function calls, and future manual prompts.

## Source Model

`SourceFile` owns the original text and exposes line spans.

Suggested types:

```rust
pub struct SourceFile {
    pub name: SourceName,
    pub text: String,
    pub line_starts: Vec<usize>,
}

pub struct Span {
    pub file_id: FileId,
    pub start: usize,
    pub end: usize,
}
```

Use byte offsets for spans and derive line and column only for display. This
keeps slicing simple and avoids doing line math throughout the parser.

Invalid UTF-8 should not block Level 0. Accept UTF-8 first. Add binary conflict
support later by introducing `SourceBytes` or a lossy decoding path that can
emit `CONFLICT (binary)`.

## Syntax Layer

The syntax layer is responsible only for recognizing MergeHell structure. It
must not execute commands or decide which lane wins.

### Line Classification

Classify each line into a small enum:

```rust
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
```

The classifier should retain original line text in the AST. Marker detection
must allow leading whitespace, but produce a warning for indented markers.
Malformed marker lengths are recognized only when `accept_regret` is enabled.

### AST

Use a compact, typed AST that keeps command labels and lane content separate.

```rust
pub struct Program {
    pub items: Vec<Node>,
    pub diagnostics: Vec<Diagnostic>,
}

pub enum Node {
    Conflict(ConflictNode),
    RawText(RawTextNode),
    Diff(DiffNode),
    Hunk(HunkNode),
    Hint(HintNode),
    Status(StatusNode),
    Error(ErrorNode),
}

pub struct ConflictNode {
    pub command: CommandHead,
    pub ours: Vec<Node>,
    pub base: Option<Lane>,
    pub theirs: Vec<Node>,
    pub metadata: Metadata,
    pub span: Span,
}

pub struct Lane {
    pub label: Option<String>,
    pub items: Vec<Node>,
    pub span: Span,
}

pub struct CommandHead {
    pub name: String,
    pub args: Vec<String>,
    pub raw: String,
}

pub struct Metadata {
    pub raw: String,
    pub tokens: Vec<String>,
}
```

Raw lines outside a conflict should remain in the AST so commands and future
modes can decide whether they are strings, metadata, or ignored content.

### Parser

The parser is line-oriented and stack-based:

1. Create a root frame.
2. On `<<<<<<<`, push a conflict frame with current lane `ours`.
3. On `|||||||`, switch the top frame to `base`.
4. On `=======`, switch the top frame to `theirs`.
5. On `>>>>>>>`, pop the top frame and append a `Conflict` node to the parent.
6. Append non-marker lines to the current lane or root as typed nodes.
7. On EOF with open frames, emit a syntax conflict diagnostic and recover by
   keeping the unfinished conflict as an `Error` node.

The parser should never use recursion to parse nested conflicts. It should only
recurse later when evaluating the AST.

## Diagnostics

Internally, diagnostics are structured:

```rust
pub enum Severity {
    Warning,
    Error,
}

pub enum DiagnosticKind {
    Syntax,
    Runtime,
    Type,
    Binary,
    Warning,
}

pub struct Diagnostic {
    pub kind: DiagnosticKind,
    pub severity: Severity,
    pub message: String,
    pub span: Option<Span>,
    pub expected: Option<String>,
    pub actual: Option<String>,
    pub hints: Vec<String>,
}
```

Rendering diagnostics should support:

- human-readable text for development if requested;
- MergeHell source snippets for normal compiler and runtime output.

Default error rendering should use the spec shape:

```txt
CONFLICT (syntax): Merge conflict in parser
<<<<<<< expected
...
=======
...
>>>>>>> parser
```

## Resolution Layer

Resolution is a policy decision. Runtime execution asks a strategy which lanes
to evaluate for each conflict.

```rust
pub enum LaneId {
    Ours,
    Base,
    Theirs,
}

pub enum Resolution {
    Lanes(Vec<LaneId>),
    Abort(Diagnostic),
}

pub trait Resolver {
    fn resolve(
        &mut self,
        conflict: &ConflictNode,
        ctx: &ResolutionContext,
    ) -> Resolution;
}
```

Required resolvers:

- `OursResolver`
- `TheirsResolver`
- `BaseResolver`
- `UnionResolver`
- `RandomResolver`
- `ManualResolver` as a later interactive implementation
- `GitResolver` as a later repository-aware implementation
- `BlameResolver` as a later implementation-defined strategy

`random` must accept an explicit seed and should derive a seed from `index`
metadata when available. Tests should never rely on unseeded randomness.

## Runtime Layer

The runtime evaluates AST nodes into values and side effects. It owns variable
bindings, functions, imports, output buffers, and current strategy state.

```rust
pub struct Runtime {
    pub resolver: Box<dyn Resolver>,
    pub commands: CommandRegistry,
    pub modules: ModuleLoader,
    pub options: RuntimeOptions,
}

pub struct Context {
    scopes: Vec<Scope>,
    functions: HashMap<String, FunctionValue>,
    stdout: OutputSink,
    stderr: OutputSink,
}
```

Evaluation should return a control-aware result:

```rust
pub enum EvalOutcome {
    Value(Value),
    Return(Value),
    Thrown(Value),
}
```

This avoids encoding `return` and `throw` as ordinary strings.

### Values

Start with these values:

```rust
pub enum Value {
    String(String),
    Number(Number),
    Bool(bool),
    Null,
    Conflict(ConflictValue),
    Blob(Vec<u8>),
    Regret(String),
}
```

Numbers can be parsed lazily when a command needs them. For Level 0 and most
Level 1 commands, raw text values are enough.

### String Interpolation

`print` and `let` examples use `${name}` interpolation. Implement this as a
runtime helper over `Value::String`:

```rust
fn interpolate(input: &str, ctx: &Context) -> Result<String, Diagnostic>;
```

Do not turn interpolation into a full expression language in the first pass.
For `if`, start with identifiers and literal booleans, then add richer
conditions only when tests require them.

## Command Layer

Commands are dispatched by opening marker command name. Unknown commands should
default to evaluating the selected lane as raw text or return a runtime conflict
depending on CLI strictness.

```rust
pub trait Command {
    fn execute(
        &self,
        conflict: &ConflictNode,
        selected: Vec<LaneId>,
        runtime: &mut Runtime,
        ctx: &mut Context,
    ) -> Result<EvalOutcome, Diagnostic>;
}
```

Required commands should be added incrementally:

- `print`: evaluate selected lanes, stringify, interpolate, write to stdout.
- `let`: evaluate selected lane to a value, optionally check base-lane type,
  bind variable.
- `if`: evaluate condition, choose ours or theirs independent of global lane
  resolution for the branch body.
- `repeat`: parse count, evaluate selected lane body repeatedly.
- `function`: store a function body as AST plus parameter names.
- `call`: evaluate call lane arguments, bind params in a new scope, evaluate
  function.
- `return`: produce `EvalOutcome::Return`.
- `try`: evaluate ours, recover with theirs on thrown or runtime conflict, run
  base as cleanup when present.
- `throw`: produce `EvalOutcome::Thrown`.
- `import`: resolve a path, parse another source file, evaluate it in module
  scope.
- `resolve`: explicitly evaluate a nested conflict with a named strategy.

The command registry should be data-driven so test builds can register only
minimal commands and future standard library modules can add commands.

## CLI Layer

Use `clap` for subcommands and flags.

Required CLI commands from the spec:

```txt
mergehell run FILE [--ours|--theirs|--base|--union|--random|--manual|--git|--blame]
mergehell check FILE
mergehell ast FILE
mergehell merge BASE OURS THEIRS
mergehell format FILE
mergehell regret FILE
```

Implementation order should prioritize:

1. `run`
2. `check`
3. `ast`
4. `merge`
5. `format`
6. `regret`

CLI code should stay thin. It reads inputs, creates options, calls library APIs,
writes stdout and stderr, and maps diagnostics to exit codes.

## Git Layer

Git integration should be isolated behind traits so the interpreter still works
outside a repository.

```rust
pub trait GitState {
    fn current_branch(&self) -> Option<String>;
    fn is_merge_in_progress(&self) -> bool;
    fn is_rebase_in_progress(&self) -> bool;
    fn is_dirty(&self) -> bool;
    fn is_detached_head(&self) -> bool;
}
```

The default implementation can shell out to `git` or inspect `.git` directly.
Keep this behind a small adapter and use fake implementations in tests.

Patch and diff parsing should initially preserve metadata nodes and later add
line-prefix semantics for `+`, `-`, and context lines.

## Formatting Layer

The formatter must preserve semantics. It should operate on the AST, not by
regex rewriting raw text.

Initial formatter scope:

- normalize marker length to seven characters;
- preserve command labels and closing metadata;
- preserve lane order;
- optionally add conflict banners and diff headers behind flags.

The intentionally worse formatting behavior should be opt-in until the core
formatter has stable round-trip tests.

## Testing Strategy

Use fixture-based integration tests because the language is line-oriented.

Test categories:

- parser fixtures: AST snapshots for basic, diff3, nested, and malformed input;
- strategy fixtures: `ours`, `theirs`, `base`, `union`, seeded `random`;
- command fixtures: stdout, stderr, exit code;
- diagnostics fixtures: syntax, runtime, type conflicts rendered as `.mh`;
- CLI golden tests: `run`, `check`, `ast`;
- Git tests: fake Git state first, real temporary Git repositories later.

Prefer exact golden outputs for user-visible behavior. Use AST debug snapshots
only if the debug representation is intentionally stable.

## Dependency Choices

Recommended initial dependencies:

- `clap`: CLI parsing.
- `thiserror`: internal Rust error ergonomics.
- `serde` and `serde_json`: AST output for `mergehell ast --json`.
- `rand` and `rand_chacha`: seeded `random` strategy.
- `assert_cmd`, `predicates`, `insta`, `tempfile`: tests.

Avoid parser generators initially. The grammar is line-oriented and stack-based,
so a handwritten parser will be clearer and easier to recover from.

## Stability Boundaries

Public library APIs should be:

- `parse_source(source: SourceFile, options: ParseOptions) -> Program`
- `run_program(program: &Program, options: RuntimeOptions) -> RunResult`
- `check_program(program: &Program, options: CheckOptions) -> CheckResult`
- `format_program(program: &Program, options: FormatOptions) -> String`

Everything else can remain internal until real use cases need it.

## Deferred Features

These should not block the MVP:

- binary conflict execution;
- real `manual` prompt UI;
- Git `blame` strategy;
- rerere persistence;
- async command execution;
- complete expression language;
- production-ready type system;
- CI integrations;
- self-modifying conflicts.

