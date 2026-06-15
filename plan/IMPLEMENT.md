# MergeHell Implementation Plan

This plan breaks the Rust implementation into phases that track the compliance
levels in `SPEC.md`. Each phase should leave the project in a runnable state.

## Phase 0: Project Skeleton

Goal: create a Rust project that can compile, run tests, and expose clean
library boundaries before language behavior is added.

Tasks:

- Create a Cargo workspace with `crates/mergehell`.
- Add `mergehell` binary entrypoint.
- Add modules for `source`, `syntax`, `resolve`, `runtime`, `commands`,
  `diagnostic`, `cli`, `git`, and `format`.
- Add basic CLI wiring with placeholder subcommands.
- Add test dependencies and fixture directories.
- Add CI-friendly commands in the README or a `justfile` if desired.

Definition of done:

- `cargo test` passes.
- `mergehell --help` works.
- Empty library APIs exist for parse, run, check, ast, and format.

## Phase 1: Compliance Level 0

Goal: execute basic conflict hunks with `print`, `ours`, and `theirs`.

Spec coverage:

- basic conflict hunks;
- `ours` and `theirs` strategies;
- `print`;
- raw text as strings;
- minimal `run`, `check`, and `ast` commands.

Tasks:

- Implement `SourceFile`, `Span`, and line iteration.
- Implement line classification for `<<<<<<<`, `=======`, `>>>>>>>`, and raw
  lines.
- Implement stack-based parser for non-nested basic conflicts.
- Add AST structs for `Program`, `Node`, `ConflictNode`, raw text, command head,
  and metadata.
- Implement `OursResolver` and `TheirsResolver`.
- Implement runtime evaluation for `Program`, raw text, and conflict nodes.
- Implement `print` command.
- Implement `mergehell run FILE --ours|--theirs`.
- Implement `mergehell check FILE` with the clean-code failure:

  ```txt
  fatal: no conflict markers found
  hint: this appears to be valid software
  ```

- Implement `mergehell ast FILE` as pretty debug output or JSON.
- Add fixtures for hello world under `ours` and `theirs`.

Definition of done:

- README hello world examples produce the documented output.
- `check` succeeds for a file with a conflict and fails for clean text.
- Parser errors render as valid MergeHell-style diagnostics.

## Phase 2: Robust Parser and Source Forms

Goal: make the parser match the structural requirements in the spec.

Spec coverage:

- nested conflicts;
- base lanes;
- diff metadata;
- hunk headers;
- hints;
- status lines as preserved syntax;
- no-final-newline marker;
- indented marker warnings;
- near-conflicts behind `--accept-regret`.

Tasks:

- Replace any simple parser assumptions with a frame stack.
- Add support for `|||||||` base lanes.
- Preserve labels on all conflict markers.
- Preserve diff nodes: `diff --git`, `diff --cc`, `index`, `---`, `+++`.
- Preserve hunk headers: `@@` and `@@@`.
- Preserve `hint:`, `error:`, `CONFLICT (`, and Git status lines.
- Detect `\ No newline at end of file`.
- Add parser recovery for unclosed conflicts and misplaced lane markers.
- Add `ParseOptions { accept_regret, git_status_mode }`.
- Emit warnings for indented conflict markers.
- Add AST snapshot tests for nested, diff3, diff-wrapped, and meta-conflict
  examples.

Definition of done:

- The nested and meta-conflict examples from the spec parse into nested
  `ConflictNode`s.
- Diff-wrapped source keeps metadata in the AST.
- Malformed marker lengths fail by default and parse only with
  `--accept-regret`.

## Phase 3: Core Runtime and Level 1 Strategies

Goal: support the Level 1 runtime surface and deterministic strategy behavior.

Spec coverage:

- `base`;
- `union`;
- seeded `random`;
- `let`;
- `if`;
- `repeat`;
- `function`;
- `call`;
- interpolation;
- basic values.

Tasks:

- Implement `Value` with string, number, bool, null, conflict, blob placeholder,
  and regret placeholder variants.
- Implement `Context`, lexical scopes, variable lookup, and assignment.
- Implement string interpolation for `${name}`.
- Implement `BaseResolver`, including the no-base diagnostic.
- Implement `UnionResolver` with order `ours`, `base`, `theirs`.
- Implement seeded `RandomResolver` using explicit seed and `index` metadata.
- Implement `let name`.
- Implement primitive type parsing helpers for numbers and booleans.
- Implement `if condition` with literal bools and identifier truthiness.
- Implement `repeat n`.
- Implement `function name args...` and `call name`.
- Add fixture tests for variables, conditionals, repeat, functions, and union
  output ordering.

Definition of done:

- Compliance Level 1 examples for variables, conditionals, repeat, and function
  calls run.
- `random` can be made reproducible in tests.
- No command needs to know parser internals.

## Phase 4: Control Flow, Errors, and Types

Goal: complete the required command set and make diagnostics executable-looking.

Spec coverage:

- `return`;
- `try`;
- `throw`;
- `import`;
- `resolve`;
- diff3 typing basics;
- runtime, syntax, and type conflict rendering.

Tasks:

- Add `EvalOutcome::Value`, `Return`, and `Thrown`.
- Implement `return`.
- Implement `throw` with runtime conflict output.
- Implement `try` where ours is attempt, theirs is recovery, and base is
  cleanup/finally when present.
- Implement `import` for local `.mh` files with cycle detection.
- Implement `resolve` for nested conflicts with an explicit strategy name.
- Parse base-lane type metadata for `let`.
- Implement basic type checks for `int`, `float`, `string`, and `bool`.
- Render type mismatches as valid MergeHell snippets.
- Add exit code mapping for syntax, runtime, and type conflicts.
- Add tests that pipe diagnostic output back through the parser.

Definition of done:

- Every required command listed in section 8 has a first implementation.
- Type errors are structured diagnostics and render as MergeHell source.
- Imports cannot recursively loop forever.

## Phase 5: CLI Completeness and Developer Tooling

Goal: make the command-line tool useful for language development and examples.

Spec coverage:

- `merge`;
- `format`;
- `regret`;
- richer `ast`;
- formatter basics.

Tasks:

- Add `mergehell ast --json` using `serde`.
- Add `mergehell merge BASE OURS THEIRS` that emits canonical conflict source.
- Add `mergehell format FILE` with conservative AST-based formatting.
- Add `mergehell regret FILE` with a first explanation of selected strategies,
  metadata, and command decisions.
- Add `--strict`, `--accept-regret`, `--seed`, and diagnostic format flags.
- Add golden CLI tests using `assert_cmd`.
- Add examples under `examples/` or `tests/fixtures/programs/`.

Definition of done:

- All reference CLI subcommands exist.
- `format` preserves semantics for Level 0 and Level 1 programs.
- `regret` can explain at least selected lanes and command dispatch.

## Phase 6: Compliance Level 2 Git Integration

Goal: add Git-aware behavior without making normal file execution depend on Git.

Spec coverage:

- custom merge driver;
- custom diff driver guidance;
- patch execution;
- Git status mode;
- `git` strategy mode.

Tasks:

- Add `GitState` trait and a real adapter for local repositories.
- Add fake Git state for tests.
- Implement `GitResolver` heuristics:
  - main branch prefers ours;
  - feature branch prefers theirs;
  - rebase in progress prefers theirs then ours;
  - merge in progress uses manual or a clear unsupported diagnostic;
  - dirty tree prefers union;
  - clean tree fails with `fatal: program is clean`;
  - detached HEAD uses seeded random;
  - bisecting delegates to blame when implemented.
- Add status-mode parsing that turns status lines into runtime metadata.
- Add patch execution for stdin and diff input.
- Interpret `+`, `-`, and context lines inside patch source for forward and
  rollback behavior.
- Implement `mergehell merge BASE OURS THEIRS` as a Git merge-driver-compatible
  command shape.
- Document `.gitattributes` and Git config snippets.
- Add tests using temporary Git repositories for merge and rebase states.

Definition of done:

- `mergehell run - --git` can read a diff from stdin.
- Git strategy works in temporary repositories for representative states.
- The merge driver command emits canonical conflict hunks and does not resolve
  them away.

## Phase 7: Compliance Level 3 Extensions

Goal: add advanced features that are useful but not required for a coherent
interpreter.

Spec coverage:

- blame strategy;
- rerere standard library;
- executable diagnostics as a workflow;
- binary conflicts;
- CI integration;
- formatter that worsens files.

Tasks:

- Implement `BlameResolver` behind a Git capability check.
- Add a small standard library registry for `rerere`, `stash`, `blame`,
  `bisect`, `reset`, `reflog`, and `submodule` module names.
- Implement rerere persistence with an explicit cache directory.
- Add binary conflict detection and `CONFLICT (binary)` diagnostics.
- Add `format --worse` to insert optional banners, fake hunk headers, and
  canonical diff-style metadata while preserving AST semantics.
- Add CI examples that run `check`, `run`, and fixture tests.
- Add documentation for diagnostic-driven development.

Definition of done:

- Blame-based resolution has deterministic tests with mocked blame output.
- Repeated conflicts can be resolved through rerere cache behavior.
- Binary conflict input produces a structured blob or diagnostic instead of a
  panic.

## Phase 8: Hardening and Release

Goal: make the implementation maintainable and distributable.

Tasks:

- Audit all parser recovery paths for panics.
- Add fuzz tests for marker-heavy input.
- Add property tests for parse-format-parse stability.
- Add performance tests for large conflicted files.
- Add user documentation for each CLI command.
- Add release packaging metadata.
- Add examples for each compliance level.
- Confirm public API boundaries and hide unstable internals.

Definition of done:

- No known parser panic on arbitrary UTF-8 text.
- Golden tests cover the examples in `README.md` and key examples in `SPEC.md`.
- Release artifacts can be built from a clean checkout.

## Suggested Milestones

Milestone 1: Level 0 MVP

- Phases 0 and 1.
- Users can run hello world with `--ours` and `--theirs`.

Milestone 2: Usable Interpreter

- Phases 2 and 3.
- Nested conflicts, base lanes, variables, conditionals, loops, and functions
  work.

Milestone 3: Required Language Surface

- Phases 4 and 5.
- Required commands and reference CLI are present.

Milestone 4: Git-Aware Interpreter

- Phase 6.
- Diff and repository state become meaningful runtime inputs.

Milestone 5: Advanced Incident

- Phases 7 and 8.
- Level 3 features, hardening, and release work.

## Development Rules

- Add parser tests before changing parser recovery behavior.
- Add command fixtures before adding new command semantics.
- Keep random behavior seedable.
- Keep Git behavior behind traits with fake implementations in tests.
- Do not make runtime execution depend on a real Git repository.
- Render user-facing errors as valid MergeHell unless a CLI flag asks for plain
  human diagnostics.
- Prefer preserving source text over normalizing it during parse.

