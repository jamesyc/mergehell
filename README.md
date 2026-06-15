# MergeHell

MergeHell is a programming language whose source code looks like...
- an unresolved Git merge conflict
- a broken patch
- a failed rebase transcript
- or some other repository incident that should probably not have been committed.

Where most tools see conflict markers and ask how to resolve them, MergeHell asks which side to execute.

```txt
<<<<<<< print
Hello, world!
=======
Goodbye, world!
>>>>>>> print
```

Run with the `ours` strategy:

```bash
mergehell run hello.mh --ours
```

Output:

```txt
Hello, world!
```

Run with the `theirs` strategy:

```bash
mergehell run hello.mh --theirs
```

Output:

```txt
Goodbye, world!
```

## What Is A MergeHell Program?

A MergeHell program is made from Git conflict markers:

```txt
<<<<<<< command arg1 arg2
ours-body
||||||| base metadata
base-body
=======
theirs-body
>>>>>>> closing metadata
```

The base lane is optional:

```txt
<<<<<<< print
hello
=======
goodbye
>>>>>>> feature/greeting
```

The opening marker contains the command, while the closing marker contains metadata; the interpreter evaluates one or more lanes according to a resolution strategy.

## Resolution Strategies

MergeHell treats conflict resolution as runtime behavior.

| Strategy | Behavior |
| --- | --- |
| `ours` | Evaluate the `<<<<<<<` lane |
| `theirs` | Evaluate the `=======` lane |
| `base` | Evaluate the `|||||||` lane |
| `union` | Evaluate all available lanes in order |
| `random` | Choose a lane randomly |
| `manual` | Ask the user |
| `git` | Infer behavior from repository state |
| `blame` | Choose using authorship metadata |

Deterministic strategies such as `ours` and `theirs` are useful for testing.
Less deterministic strategies are useful for regret.

## Commands

The baseline language includes these commands:

| Command | Meaning |
| --- | --- |
| `print` | Print the resolved body |
| `let name` | Bind a variable |
| `if condition` | Evaluate one lane conditionally |
| `repeat n` | Repeat the selected body |
| `function name args...` | Define a function |
| `call name` | Call a function |
| `return` | Return a value |
| `try` | Attempt one lane and recover with another |
| `throw` | Raise a runtime conflict |
| `import` | Import another conflicted file |
| `resolve` | Explicitly resolve a nested conflict |

Raw text is generally treated as a string until a command decides otherwise.

## Variables

```txt
<<<<<<< let name
James
||||||| string default
User
=======
process.env.USER
>>>>>>> feature/env-name

<<<<<<< print
Hello, ${name}
=======
Goodbye, ${name}
>>>>>>> print
```

Under `--ours`, this binds `name` to `James` and prints:

```txt
Hello, James
```

Under `--base`, the base lane may provide a type, default value, documentation, or prior state.

## Nested Conflicts

Real Git conflicts inside MergeHell source are valid syntax!

```txt
<<<<<<< HEAD
<<<<<<< print
Hello
=======
Goodbye
>>>>>>> print
=======
<<<<<<< print
Hola
=======
Adios
>>>>>>> print
>>>>>>> feature/spanish
```

This is a conflict between two programs, each of which is also a conflict.
MergeHell parses it as nested conflict nodes instead of rejecting it as an unresolved file.

## Diff-Wrapped Source

MergeHell source may also look like a Git diff:

```txt
diff --git a/stdin b/stdout
index deadbee..c0ffee0
--- a/stdin
+++ b/stdout
@@ -1,7 +1,12 @@ main

<<<<<<< print
Hello
=======
Goodbye
>>>>>>> feature/greeting
```

Diff metadata is accepted as program metadata. In more advanced modes, it may provide module names, source ranges, random seeds, or input/output channels.

## Diagnostics

Errors in MergeHell are called conflicts, and diagnostics should be valid MergeHell source:

```txt
CONFLICT (syntax): Merge conflict in parser
<<<<<<< expected
>>>>>>>
=======
end of file
>>>>>>> parser
```

This means a failed build can produce another MergeHell program.

## Git Integration

MergeHell is intentionally designed to work very badly inside Git on purpose.

Recommended `.gitattributes`:

```gitattributes
*.mh merge=mergehell
*.mh diff=mergehell
```

Recommended merge driver shape:

```ini
[merge "mergehell"]
    name = MergeHell conflict-preserving merge driver
    driver = mergehell merge %O %A %B %L %P
```

The merge driver should preserve and canonicalize conflicts rather than resolve them.

## CLI Reference

```bash
mergehell run FILE [--ours|--theirs|--base|--union|--random|--git] [--seed N] [--accept-regret] [--strict]
mergehell check FILE [--accept-regret] [--strict]
mergehell ast FILE [--json] [--accept-regret]
mergehell format FILE [--worse]
mergehell merge BASE OURS THEIRS
mergehell regret FILE
```

Examples:

```bash
mergehell run examples/level0_hello.mh --ours
mergehell run examples/level1_variables.mh --base
mergehell run examples/level2_patch.mh --ours
mergehell ast examples/level0_hello.mh --json
mergehell format examples/level0_hello.mh --worse
```

`run -` reads from stdin, including diff-like input. Diff input strips `+`
prefixes, skips `-` lines in forward execution, and preserves context lines.

## Development

Run the local verification suite with:

```bash
cargo fmt --all
cargo test --all
```

## Non-Goals

MergeHell does not aim to be readable, safe, easy to lint, compatible with normal development workflows, or accepted by code review.

It does aim to be technically coherent, Git-shaped, executable, and able to turn real merge conflicts into language constructs.

## Specification

Read [SPEC.md](SPEC.md) for the full language specification, including the grammar, evaluation model, compliance levels, Git integration notes, and examples.
