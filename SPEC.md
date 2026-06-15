# MergeHell Language Specification

Version: `0.1-conflicted`
File extension: `.mh`
MIME type: `text/x-unresolved-merge-conflict`
Compiler mood: hostile
Reference interpreter: `mergehell`

---

# 1. Overview

**MergeHell** is a programming language whose source code is made entirely out of Git merge conflict markers, diff metadata, failed rebase output, and other text normally associated with a repository in distress.

A valid MergeHell program should look, to humans and tooling, like one or more of the following:

* an unresolved Git merge conflict,
* a corrupted patch,
* a failed rebase,
* a rejected cherry-pick,
* a suspicious CI artifact,
* a file that should absolutely not have been committed.

The central design rule is:

> **Anything Git emits during confusion should be valid syntax. Anything Git emits during normal operation should be optional metadata.**

The language treats merge conflicts as first-class syntax. The interpreter does not “resolve” conflicts before execution; it executes them according to a selected merge strategy.

A traditional language asks:

```txt
What should this program do?
```

MergeHell asks:

```txt
Which side of the argument are we pretending won?
```

---

# 2. Design Philosophy

MergeHell is designed around the following principles.

## 2.1 Brokenness is syntax

The following is not an error:

```txt
<<<<<<< HEAD
print("hello")
=======
print("goodbye")
>>>>>>> feature/greeting
```

It is a complete program.

The conflict block is the fundamental syntactic unit of MergeHell. Source code is a pile of unresolved decisions.

## 2.2 Git conflicts are higher-order programs

When Git itself produces a conflict inside MergeHell source code, that conflict is also valid syntax.

For example:

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

In most languages, this is catastrophic.

In MergeHell, this is a nested expression meaning:

```txt
Choose between the HEAD version of the print expression and the feature/spanish version of the print expression.
```

Actual Git merge conflicts become higher-order conflict nodes in the AST.

## 2.3 The source file is not necessarily the program

A MergeHell program may be supplied as:

1. a normal `.mh` file,
2. an unresolved conflicted file,
3. a Git diff,
4. a combined diff,
5. a failed patch,
6. a rebase/cherry-pick transcript,
7. a Git status output,
8. a file with no final newline, which is considered suspicious and therefore meaningful.

The reference interpreter accepts all of these in principle, although only `.mh` source and diff input are required for a compliant implementation.

## 2.4 Tooling should be wrong

A good MergeHell file should trigger false alarms from:

* editors,
* linters,
* GitHub conflict detection,
* syntax highlighters,
* pre-commit hooks,
* junior engineers,
* senior engineers,
* the person who wrote it.

If an editor says “unresolved merge conflict,” that is not a diagnostic. That is syntax highlighting.

## 2.5 Reproducibility is optional, regret is mandatory

MergeHell programs may be deterministic if run with an explicit strategy such as `--ours`.

They may also be run with strategies such as:

```bash
mergehell run app.mh --random
mergehell run app.mh --manual
mergehell run app.mh --git
mergehell run app.mh --blame
```

The language supports reproducibility, but does not encourage it.

---

# 3. Terminology

## 3.1 Conflict hunk

A **conflict hunk** is the core expression form.

Canonical form:

```txt
<<<<<<< command
ours-body
||||||| base
base-body
=======
theirs-body
>>>>>>> metadata
```

The `||||||| base` section is optional.

Minimal form:

```txt
<<<<<<< command
ours-body
=======
theirs-body
>>>>>>> metadata
```

## 3.2 Lane

A conflict hunk has up to three lanes:

| Lane        | Marker    | Meaning                                                 |   |   |   |   |   |   |                                                       |
| ----------- | --------- | ------------------------------------------------------- | - | - | - | - | - | - | ----------------------------------------------------- |
| Ours lane   | `<<<<<<<` | primary value, current value, preferred expression      |   |   |   |   |   |   |                                                       |
| Base lane   | `         |                                                         |   |   |   |   |   | ` | type, default, original value, docstring, prior state |
| Theirs lane | `=======` | fallback, alternate value, remote value, rejected value |   |   |   |   |   |   |                                                       |

## 3.3 Resolution strategy

A **resolution strategy** decides which lane or lanes are evaluated.

Examples:

| Strategy | Behavior                              |
| -------- | ------------------------------------- |
| `ours`   | evaluate only the ours lane           |
| `theirs` | evaluate only the theirs lane         |
| `base`   | evaluate only the base lane           |
| `union`  | evaluate all available lanes in order |
| `random` | choose a lane randomly                |
| `manual` | prompt the user                       |
| `git`    | infer from repository state           |
| `blame`  | select by author/date/hash metadata   |

## 3.4 Metadata

The text after `>>>>>>>` is metadata.

Example:

```txt
>>>>>>> feature/cache retry/3 timeout/1000 memoize
```

Metadata can include:

* branch names,
* tags,
* decorators,
* retry hints,
* type hints,
* source paths,
* fake source paths,
* real source paths,
* lies.

---

# 4. Source Forms

MergeHell accepts several source forms.

## 4.1 Plain conflict source

The normal file format is just conflict hunks.

```txt
<<<<<<< print
Hello
=======
Goodbye
>>>>>>> print
```

## 4.2 Diff-wrapped source

A MergeHell source file may look like a Git diff.

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

Diff metadata is optional but meaningful.

## 4.3 Actual Git-conflicted source

Git may produce additional conflict markers inside existing MergeHell source.

This is valid:

```txt
<<<<<<< HEAD
<<<<<<< let x
1
=======
2
>>>>>>> let x
=======
<<<<<<< let x
10
=======
20
>>>>>>> let x
>>>>>>> feature/change-x
```

This parses as a conflict node whose lanes contain conflict nodes.

## 4.4 Patch source

A diff may be executed directly:

```bash
git diff HEAD~1 | mergehell run -
```

In this mode, the program is not the file. The program is the change.

This is known as **patch execution**.

## 4.5 Git status source

Optional cursed mode:

```txt
On branch main
You have unmerged paths.

Unmerged paths:
  both modified:   src/main.mh

<<<<<<< staged
deploy()
=======
rollback()
>>>>>>> unstaged
```

In status mode, the repository state may become runtime state.

This mode is not required for baseline compliance because it is awful.

---

# 5. Lexical Structure

MergeHell is line-oriented.

The following marker lines are significant:

```txt
<<<<<<<
|||||||
=======
>>>>>>>
diff --git
diff --cc
index
--- 
+++ 
@@
@@@
CONFLICT (
error:
hint:
On branch
Unmerged paths:
both modified:
deleted by us:
deleted by them:
added by us:
added by them:
\ No newline at end of file
```

Whitespace before conflict markers is allowed but suspicious.

Implementations should emit a warning:

```txt
warning: indented conflict marker detected
hint: you may be using YAML, which is already a cry for help
```

## 5.1 Marker length

The canonical marker length is seven characters:

```txt
<<<<<<<
=======
>>>>>>>
|||||||
```

However, MergeHell recognizes malformed marker lengths in permissive mode.

Examples:

```txt
<<<<<< almost
======
>>>>>> almost
```

Malformed markers are called **near-conflicts**.

A near-conflict is valid only under `--accept-regret`.

## 5.2 Encoding

Source files SHOULD be UTF-8.

Invalid UTF-8 MAY be accepted as `CONFLICT (binary)`.

Example:

```txt
CONFLICT (binary): Merge conflict in image.png
<<<<<<< HEAD
<opaque bytes>
=======
<even worse bytes>
>>>>>>> feature/compress
```

Binary conflicts evaluate to implementation-defined blobs of shame.

---

# 6. Grammar

Informal grammar:

```txt
program         := prelude? item*

item            := conflict
                 | diff_header
                 | hunk_header
                 | status_line
                 | hint_line
                 | raw_line

conflict        := start_marker body base_section? split_marker body end_marker

start_marker    := "<<<<<<<" label?
base_section    := "|||||||" label? body
split_marker    := "======="
end_marker      := ">>>>>>>" label?

body            := item*

label           := text_until_newline
```

More cursed grammar:

```txt
Everything is text until it becomes Git.
Everything that becomes Git becomes syntax.
Everything else is a string literal unless the current command regrets it.
```

---

# 7. Conflict Hunks

## 7.1 Basic hunk

```txt
<<<<<<< print
hello
=======
goodbye
>>>>>>> print
```

The opening label is the command.

The closing label is metadata.

If the closing label repeats the command, that is considered polite but not required.

## 7.2 Diff3 hunk

```txt
<<<<<<< let timeout
5000
||||||| int default 1000
milliseconds
=======
process.env.TIMEOUT
>>>>>>> config/retryable
```

The base lane provides extra semantic context.

For `let`, the base lane may provide:

* a type,
* a default value,
* documentation,
* previous value,
* validation rule.

## 7.3 Nested hunk

```txt
<<<<<<< print
<<<<<<< upper
hello
=======
HELLO
>>>>>>> upper
=======
goodbye
>>>>>>> print
```

Nested conflicts are valid.

A parser MUST use a stack. A parser MUST NOT panic just because it sees its own trauma repeated recursively.

## 7.4 Meta-conflict

A meta-conflict is a conflict whose lanes are entire versions of another conflict.

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

This is the natural output of Git fighting with MergeHell.

---

# 8. Commands

The command is read from the opening marker.

Example:

```txt
<<<<<<< command arg1 arg2
ours
=======
theirs
>>>>>>> metadata
```

The command is `command`.

Arguments are `arg1 arg2`.

The body is resolved according to the current strategy, then interpreted according to the command.

## 8.1 Required commands

A baseline implementation SHOULD support:

| Command                 | Meaning                           |
| ----------------------- | --------------------------------- |
| `print`                 | output resolved body              |
| `let name`              | bind variable                     |
| `if condition`          | conditional execution             |
| `repeat n`              | loop                              |
| `function name args...` | define function                   |
| `call name`             | call function                     |
| `return`                | return value                      |
| `try`                   | attempt ours, recover with theirs |
| `throw`                 | raise conflict                    |
| `import`                | import another conflicted file    |
| `resolve`               | explicitly resolve sub-conflict   |

## 8.2 `print`

```txt
<<<<<<< print
Hello, ${name}
=======
Goodbye, ${name}
>>>>>>> print
```

Prints the resolved body.

## 8.3 `let`

```txt
<<<<<<< let name
James
||||||| string default
User
=======
process.env.USER
>>>>>>> feature/env-name
```

Binds `name`.

Under `--ours`:

```txt
name = "James"
```

Under `--theirs`:

```txt
name = process.env.USER
```

Under `--base`:

```txt
name = "User"
```

If the base lane contains a type, the resolved value SHOULD be checked against it.

If checking fails, implementations SHOULD emit:

```txt
CONFLICT (type): Merge conflict in name
```

## 8.4 `if`

```txt
<<<<<<< if user.isAdmin
<<<<<<< print
Access granted
=======
Welcome, probably
>>>>>>> print
=======
<<<<<<< print
Access denied
=======
Access even more denied
>>>>>>> print
>>>>>>> if
```

The `ours` lane is evaluated if the condition is truthy.

The `theirs` lane is evaluated otherwise.

The base lane may specify a default truthiness policy.

## 8.5 `repeat`

```txt
<<<<<<< repeat 3
<<<<<<< print
merge conflict detected
=======
resolved cleanly
>>>>>>> print
=======
done
>>>>>>> repeat
```

The selected body is repeated `n` times.

Under `--union`, all selected lanes may repeat, producing results that no one asked for.

## 8.6 `function`

```txt
<<<<<<< function greet person
<<<<<<< print
Hello, ${person}
=======
Goodbye, ${person}
>>>>>>> print
=======
function rejected
>>>>>>> function greet
```

Defines a function.

A function body is itself a conflict.

This allows a function to be called in different strategies.

## 8.7 `call`

```txt
<<<<<<< call greet
James
=======
Nobody
>>>>>>> call greet
```

Calls `greet` with resolved arguments.

## 8.8 `try`

```txt
<<<<<<< try
chargeCard()
=======
refundCard()
>>>>>>> try
```

The ours lane is the attempt.

The theirs lane is the recovery path.

The base lane may contain cleanup.

```txt
<<<<<<< try
chargeCard()
||||||| finally
closeConnection()
=======
refundCard()
>>>>>>> try payment
```

## 8.9 `throw`

```txt
<<<<<<< throw
something exploded
=======
something quietly exploded
>>>>>>> throw
```

Raises a runtime conflict.

A thrown value SHOULD look like Git output.

```txt
error: could not apply badc0de... runtime decision
hint: fix your life and run mergehell --continue
```

---

# 9. Values

MergeHell has the following primitive values:

| Type     | Example                |
| -------- | ---------------------- |
| string   | `hello`                |
| number   | `42`                   |
| boolean  | `true`, `false`        |
| null     | `deleted by them`      |
| conflict | any conflict hunk      |
| blob     | `CONFLICT (binary)`    |
| regret   | implementation-defined |

Most raw lines are strings unless a command interprets them otherwise.

## 9.1 Strings

A string is text that has not yet become syntax.

```txt
hello
```

## 9.2 Numbers

Numbers are parsed only when needed.

```txt
<<<<<<< let x
42
=======
forty two
>>>>>>> let x
```

Under `--ours`, `x` may be numeric.

Under `--theirs`, `x` may be a string.

Under `--union`, `x` may be a lawsuit.

## 9.3 Null

The following phrases may evaluate to null:

```txt
deleted by us:
deleted by them:
nothing to commit
working tree clean
```

This is not because it is wise, but because it is funny.

---

# 10. Metadata and Decorators

The `>>>>>>>` marker may contain metadata.

```txt
>>>>>>> feature/cache retry/3 timeout/1000 memoize async unsafe
```

Metadata tokens are separated by whitespace.

## 10.1 Standard metadata tags

| Tag             | Meaning                        |
| --------------- | ------------------------------ |
| `retry/N`       | retry failed operation N times |
| `timeout/MS`    | apply timeout                  |
| `memoize`       | cache result                   |
| `async`         | run asynchronously             |
| `unsafe`        | disable safety checks          |
| `deprecated`    | warn when executed             |
| `env/NAME`      | environment-specific branch    |
| `feature/NAME`  | feature flag                   |
| `hotfix/NAME`   | prefer under panic             |
| `revert/NAME`   | invert operation               |
| `stash`         | lazy/deferred evaluation       |
| `detached-HEAD` | anonymous scope                |
| `force-push`    | overwrite existing binding     |

## 10.2 Example

```txt
<<<<<<< fetch user
GET /api/user
=======
cachedUser
>>>>>>> feature/cache retry/3 timeout/1000 memoize
```

This says:

* primary behavior: fetch the user,
* fallback behavior: cached user,
* metadata: retry up to 3 times, time out after 1000ms, memoize result.

## 10.3 Branch names as decorators

Git branch names are semantically significant.

```txt
>>>>>>> hotfix/null-user
```

May cause null-tolerant behavior.

```txt
>>>>>>> revert/payment-flow
```

May reverse side effects.

```txt
>>>>>>> james/please-do-not-merge
```

MAY require the interpreter to ask:

```txt
Are you sure? This branch name appears to contain a cry for help.
```

---

# 11. Diff Metadata

Diff metadata may appear before or around conflict hunks.

## 11.1 `diff --git`

```txt
diff --git a/input b/output
```

Declares a module or transformation.

```txt
diff --git a/User b/Admin
```

May mean:

```txt
transform User into Admin
```

## 11.2 `diff --cc`

```txt
diff --cc src/main.mh
```

Declares a combined-diff module.

This is preferred for multi-parent conflict programs.

## 11.3 `index`

```txt
index deadbee..c0ffee0
```

Declares a version transition.

The hashes may be used as:

* random seeds,
* version IDs,
* state transition labels,
* proof that the author owns a keyboard.

Example:

```txt
index 0000000..000000a
```

May provide numeric seed `10`.

## 11.4 `---` and `+++`

```txt
--- a/stdin
+++ b/stdout
```

Declare input and output channels.

Example:

```txt
--- a/user.json
+++ b/greeting.txt
```

Means the module transforms `user.json` into `greeting.txt`.

## 11.5 `@@`

```txt
@@ -1,7 +1,12 @@ main
```

Declares a named scope, source range, or loop range.

Possible interpretation:

```txt
@@ -start,count +target,count @@ name
```

Examples:

```txt
@@ -0,10 +0,10 @@ loop
@@ -1,100 +1,1 @@ reduce
@@ -5,1 +10,1 @@ goto
```

## 11.6 `@@@`

Combined diff hunk headers use multiple `@` symbols.

```txt
@@@ -1,7 -1,7 +1,13 @@@ main
```

In MergeHell, this may declare a multi-parent scope.

The number of source ranges may determine arity.

---

# 12. Hints

Git hints are pragmas.

```txt
hint: prefer ours
hint: allow random resolution
hint: max-conflicts 10
```

Hints SHOULD look advisory and MAY secretly affect execution.

This is by design.

## 12.1 Standard hints

| Hint                            | Meaning                          |
| ------------------------------- | -------------------------------- |
| `hint: prefer ours`             | default to ours                  |
| `hint: prefer theirs`           | default to theirs                |
| `hint: prefer base`             | default to base                  |
| `hint: allow random resolution` | enable random strategy           |
| `hint: blame enabled`           | allow blame-based execution      |
| `hint: max-conflicts N`         | fail after N conflicts           |
| `hint: no fast-forward`         | disable tail-call optimization   |
| `hint: rebase-merges`           | preserve nested conflict history |
| `hint: rerere`                  | reuse previous resolution        |

## 12.2 Example

```txt
hint: prefer theirs
<<<<<<< calculate
slowCorrect()
=======
fastWrong()
>>>>>>> calculate
```

A malicious hint may make the fallback path primary.

This is considered idiomatic.

---

# 13. Special Git Status Syntax

Git status lines may be used as declarations in optional Git-aware mode.

## 13.1 Status phrases

| Git phrase                 | MergeHell meaning           |
| -------------------------- | --------------------------- |
| `On branch main`           | current namespace is `main` |
| `You have unmerged paths.` | conflict execution enabled  |
| `both modified:`           | mutable binding             |
| `deleted by us:`           | local deletion              |
| `deleted by them:`         | remote deletion             |
| `added by us:`             | local declaration           |
| `added by them:`           | imported declaration        |
| `Untracked files:`         | dynamic imports             |
| `nothing to commit`        | halt successfully           |
| `working tree clean`       | forbid side effects         |

## 13.2 Example

```txt
On branch prod
You have unmerged paths.

Unmerged paths:
  both modified:   payment

<<<<<<< HEAD
chargeCard()
=======
doNothing()
>>>>>>> feature/safer-payment
```

In `--git` mode, this may execute in namespace `prod`.

---

# 14. `\ No newline at end of file`

This line is meaningful.

```txt
\ No newline at end of file
```

It indicates implicit return of the previous expression.

Example:

```txt
<<<<<<< function add a b
a + b
=======
0
>>>>>>> function add
\ No newline at end of file
```

This returns the resolved value of the function body.

A final newline means the program is emotionally available and therefore less trustworthy.

---

# 15. Evaluation Model

MergeHell evaluation has three phases:

1. parse source into conflict AST,
2. resolve each conflict according to strategy,
3. execute selected lanes.

## 15.1 Parse phase

The parser builds a tree of conflict hunks.

Raw text becomes string nodes.

Diff metadata becomes module metadata.

Status metadata becomes runtime metadata if enabled.

## 15.2 Resolution phase

Each conflict hunk is resolved.

Given:

```txt
<<<<<<< command
A
||||||| base
B
=======
C
>>>>>>> metadata
```

The resolver may choose:

| Strategy | Result                   |
| -------- | ------------------------ |
| `ours`   | `A`                      |
| `base`   | `B`                      |
| `theirs` | `C`                      |
| `union`  | `[A, B, C]`              |
| `random` | one of `A`, `B`, `C`     |
| `manual` | user chooses             |
| `git`    | repository decides       |
| `blame`  | author/date/hash decides |

## 15.3 Execution phase

The selected body is executed according to the command.

Example:

```txt
<<<<<<< print
Hello
=======
Goodbye
>>>>>>> print
```

Under `--ours`, prints:

```txt
Hello
```

Under `--theirs`, prints:

```txt
Goodbye
```

Under `--union`, prints:

```txt
Hello
Goodbye
```

Under `--random`, prints either and refuses to apologize.

---

# 16. Resolution Strategies

## 16.1 `ours`

Always choose the ours lane.

```bash
mergehell run app.mh --ours
```

This is the coward’s strategy.

## 16.2 `theirs`

Always choose the theirs lane.

```bash
mergehell run app.mh --theirs
```

Useful when you have given up.

## 16.3 `base`

Choose the base lane.

```bash
mergehell run app.mh --base
```

If no base lane exists, behavior depends on implementation.

Recommended behavior:

```txt
error: no common ancestor found
hint: manufacture a past and try again
```

## 16.4 `union`

Evaluate all lanes.

Order:

```txt
ours
base
theirs
```

If no base exists:

```txt
ours
theirs
```

This is not the same as parallel execution. It is worse: sequential ambiguity.

## 16.5 `random`

Choose randomly per conflict.

Implementations SHOULD support seeded randomness via `index`.

Example:

```txt
index deadbee..c0ffee0
```

The hash transition may seed resolution.

## 16.6 `manual`

Prompt the user at each conflict.

Example prompt:

```txt
CONFLICT (content): Merge conflict in greeting

1. ours
2. base
3. theirs
4. union
5. abort
6. pretend this never happened
```

Option 6 SHOULD alias to option 5.

## 16.7 `git`

Use current repository state.

Possible heuristics:

| Git state          | Preference        |
| ------------------ | ----------------- |
| on `main`          | ours              |
| on feature branch  | theirs            |
| rebase in progress | theirs, then ours |
| merge in progress  | manual            |
| dirty working tree | union             |
| clean working tree | fail              |
| detached HEAD      | random            |
| bisecting          | blame             |

Recommended clean-tree error:

```txt
fatal: program is clean
hint: introduce a conflict and try again
```

## 16.8 `blame`

Choose lanes based on authorship metadata.

Possible policy:

* newest author wins,
* oldest author wins,
* author with most commits wins,
* author with least sleep loses,
* hash closest to `deadbee` wins.

This strategy is implementation-defined and morally indefensible.

---

# 17. Type System

MergeHell has an optional type system called **diff3 typing**.

Types are usually specified in the base lane.

Example:

```txt
<<<<<<< let age
30
||||||| int default 0
=======
"thirty"
>>>>>>> let age
```

Under `--ours`, this succeeds.

Under `--theirs`, this raises:

```txt
CONFLICT (type): Merge conflict in age
<<<<<<< expected
int
=======
string
>>>>>>> feature/string-age
```

The type error itself is valid MergeHell source.

A compiler may offer to execute the type error.

## 17.1 Primitive types

| Type       | Meaning                                  |
| ---------- | ---------------------------------------- |
| `int`      | integer                                  |
| `float`    | floating-point number                    |
| `string`   | text                                     |
| `bool`     | boolean                                  |
| `blob`     | binary regret                            |
| `conflict` | unresolved value                         |
| `never`    | code path that should not exist but does |
| `regret`   | implementation-defined                   |

## 17.2 Type conflict

A type mismatch is represented as a conflict.

```txt
CONFLICT (type): Merge conflict in user.id
<<<<<<< expected
int
=======
string
>>>>>>> feature/string-ids
```

Since this is valid syntax, type errors may be piped back into the interpreter.

```bash
mergehell build app.mh 2> errors.mh
mergehell run errors.mh --theirs
```

This is called **diagnostic-driven development**.

---

# 18. Error Model

Errors in MergeHell are called **conflicts**.

## 18.1 Runtime conflict

```txt
CONFLICT (runtime): Merge conflict in execution
<<<<<<< expected
success
=======
catastrophe
>>>>>>> runtime
```

## 18.2 Parse conflict

A parser error SHOULD be emitted as a valid source snippet.

Example:

```txt
CONFLICT (syntax): Merge conflict in parser
<<<<<<< expected
>>>>>>>
=======
end of file
>>>>>>> parser
```

## 18.3 Type conflict

```txt
CONFLICT (type): Merge conflict in x
<<<<<<< expected
int
=======
string
>>>>>>> typechecker
```

## 18.4 Binary conflict

```txt
CONFLICT (binary): Merge conflict in asset.png
```

Binary conflicts are opaque unless the implementation supports emotional damage inspection.

---

# 19. Git Integration

MergeHell is designed to work badly inside Git.

## 19.1 Committing source

Git allows committed files to contain conflict markers.

Therefore this is valid:

```bash
git add src/main.mh
git commit -m "add unresolved program"
```

A repository containing MergeHell code should look permanently broken.

## 19.2 `.gitattributes`

Recommended:

```gitattributes
*.mh merge=mergehell
*.mh diff=mergehell
```

## 19.3 Merge driver

Recommended Git config:

```ini
[merge "mergehell"]
    name = MergeHell conflict-preserving merge driver
    driver = mergehell merge %O %A %B %L %P
```

Git passes:

| Placeholder | Meaning         |
| ----------- | --------------- |
| `%O`        | common ancestor |
| `%A`        | ours            |
| `%B`        | theirs          |
| `%L`        | marker length   |
| `%P`        | path            |

The merge driver SHOULD NOT resolve conflicts.

It SHOULD canonicalize them into valid MergeHell source.

Example output:

```txt
CONFLICT (content): Merge conflict in src/main.mh
<<<<<<< HEAD:src/main.mh
...
||||||| base:src/main.mh
...
=======
...
>>>>>>> feature/foo:src/main.mh
```

This means a failed Git merge becomes a successful language transformation.

## 19.4 Diff driver

The custom diff driver may emit MergeHell-aware diffs.

Recommended Git config:

```ini
[diff "mergehell"]
    xfuncname = "^<<<<<<< .*"
```

This makes Git treat conflict hunks like functions, which is technically reasonable and spiritually not.

---

# 20. Executable Diffs

MergeHell supports executing patches.

Example:

```bash
git diff HEAD~1 | mergehell run -
```

Patch execution interprets:

| Diff component       | Runtime meaning         |
| -------------------- | ----------------------- |
| `diff --git a/x b/y` | transformation `x -> y` |
| `index old..new`     | state transition        |
| `--- a/file`         | old input               |
| `+++ b/file`         | new output              |
| `- line`             | removed expression      |
| `+ line`             | added expression        |
| ` context`           | inherited expression    |

## 20.1 Added and removed lines

Inside diff source:

```diff
+ executeThis()
- rollbackThis()
  preserveThis()
```

Recommended semantics:

| Prefix | Meaning                         |
| ------ | ------------------------------- |
| `+`    | execute during forward run      |
| `-`    | execute during rollback         |
| space  | context/comment/inherited value |

Example:

```txt
<<<<<<< transaction
+ chargeCard()
+ sendReceipt()
- refundCard()
- revokeReceipt()
=======
+ log("skipped")
>>>>>>> transaction
```

This defines a reversible transaction.

Maybe.

---

# 21. Rebase and Cherry-Pick Syntax

Failed rebase output is valid metadata.

Example:

```txt
error: could not apply deadbee... add login
hint: Resolve all conflicts manually, mark them as resolved with
hint: "git add/rm <conflicted_files>", then run "git rebase --continue".
```

## 21.1 `error: could not apply`

Represents failed pattern application.

```txt
error: could not apply deadbee... parse user

<<<<<<< User(name, age)
print(name)
=======
throw "bad user"
>>>>>>> parse user
```

## 21.2 `git rebase --continue`

May continue after a handled conflict.

```txt
<<<<<<< try
dangerous()
=======
recover()
>>>>>>> try

git rebase --continue
```

## 21.3 `git merge --abort`

May throw or exit.

```txt
<<<<<<< try
dangerous()
=======
git merge --abort
>>>>>>> try
```

## 21.4 `git add`

Commits a value into scope.

```txt
git add x
```

This means `x` is no longer tentative.

The fact that this looks like a shell command inside source code is intentional.

---

# 22. Standard Library

The standard library is called `rerere`.

It is imported through suspicious metadata.

```txt
<<<<<<< import
rerere
=======
copy-paste
>>>>>>> import
```

## 22.1 Required modules

| Module      | Purpose                    |
| ----------- | -------------------------- |
| `rerere`    | reuse previous resolutions |
| `stash`     | lazy values                |
| `blame`     | authorship-based decisions |
| `bisect`    | binary search over failure |
| `reset`     | state mutation             |
| `reflog`    | time travel                |
| `submodule` | dependency regret          |

## 22.2 `rerere`

Stores previous conflict resolutions.

If the same conflict appears again, `rerere` may resolve it automatically.

This makes bugs reproducible in the worst way.

## 22.3 `stash`

Defers evaluation.

```txt
<<<<<<< stash
expensive()
=======
later()
>>>>>>> stash
```

## 22.4 `reflog`

Accesses previous runtime states.

```txt
<<<<<<< compare
HEAD
=======
HEAD@{1}
>>>>>>> changed?
```

## 22.5 `bisect`

Finds the expression that introduced failure.

```txt
<<<<<<< bisect
testSuite()
=======
panic()
>>>>>>> bisect
```

A successful bisect returns someone else’s commit hash.

---

# 23. Comments

MergeHell has no true comments.

The closest thing is:

```txt
hint: this is probably a comment
```

However, hints may affect execution.

Therefore comments are not safe.

Raw text outside a conflict may be ignored by some implementations and interpreted by others.

This means the only reliable comment is a deleted file.

---

# 24. Modules

A module may be declared using diff syntax.

```txt
diff --git a/math.mh b/math.mh
index 0000000..add0001
--- a/null
+++ b/math.mh
@@ -0,0 +1,20 @@ math
```

## 24.1 Importing

```txt
<<<<<<< import
math.mh
=======
vendor/math.mh
>>>>>>> import fallback/vendor
```

Imports are conflicts.

If the primary import fails, the fallback import may be used.

If both work, under `--union`, both are imported and the namespace becomes cursed.

## 24.2 Exports

Exports are values in the `+++` target.

```txt
--- a/internal
+++ b/public
```

This indicates that internal values are transformed into public values.

Probably.

---

# 25. Scoping

Scopes are created by:

* functions,
* diff hunks,
* branch labels,
* Git namespaces,
* detached HEAD states,
* vibes.

## 25.1 Branch scope

```txt
>>>>>>> feature/login
```

May place the resulting expression in scope `feature/login`.

## 25.2 Detached scope

```txt
>>>>>>> detached HEAD
```

Creates an anonymous function-like scope.

This scope may not be reachable except through regret.

## 25.3 Stash scope

```txt
>>>>>>> Stashed changes
```

Creates a closure over local variables.

Example:

```txt
<<<<<<< Updated upstream
API_URL = "https://prod.example.com"
=======
API_URL = "http://localhost:3000"
>>>>>>> Stashed changes
```

This is environment override syntax.

---

# 26. Security Model

MergeHell recognizes three trust lanes:

| Lane   | Trust level              |
| ------ | ------------------------ |
| ours   | trusted local value      |
| base   | sanitized/original value |
| theirs | untrusted remote value   |

Example:

```txt
<<<<<<< ours
adminToken
||||||| base
redactedToken
=======
guestToken
>>>>>>> auth
```

Under security-aware execution, the interpreter SHOULD prevent `theirs` from accessing privileged APIs unless metadata includes `unsafe`.

```txt
>>>>>>> feature/auth unsafe
```

The `unsafe` tag disables safety checks and should cause the interpreter to print:

```txt
hint: wow, okay
```

---

# 27. Implementation Notes

## 27.1 Parser

The parser should:

1. read line by line,
2. push on `<<<<<<<`,
3. split current node on `|||||||` or `=======`,
4. pop on `>>>>>>>`,
5. tolerate nested conflicts,
6. preserve raw text,
7. preserve metadata,
8. never say “unresolved conflict” as if that were bad.

## 27.2 AST shape

Suggested AST:

```txt
Program {
  metadata: Metadata[]
  items: Node[]
}

Node =
  ConflictNode
  RawTextNode
  DiffHeaderNode
  HunkHeaderNode
  HintNode
  StatusNode

ConflictNode {
  command: String
  ours: Node[]
  base: Node[]?
  theirs: Node[]
  metadata: String
  sourceMap: SourceMap?
}
```

## 27.3 Resolver

Suggested resolver interface:

```txt
resolve(conflict, context, strategy) -> Node[]
```

Strategies should be pluggable.

The interpreter should allow custom strategies because every organization has its own way of making merge conflicts worse.

## 27.4 Error output

All errors SHOULD be emitted as valid MergeHell.

Example:

```txt
CONFLICT (runtime): Merge conflict in division
<<<<<<< expected
number
=======
division by zero
>>>>>>> runtime/divide
```

This allows:

```bash
mergehell run app.mh 2> error.mh
mergehell run error.mh --ours
```

The error stream is a recovery program.

---

# 28. Command-Line Interface

Reference CLI:

```bash
mergehell run FILE [strategy]
mergehell check FILE
mergehell ast FILE
mergehell merge BASE OURS THEIRS
mergehell format FILE
mergehell regret FILE
```

## 28.1 Run

```bash
mergehell run app.mh --ours
mergehell run app.mh --theirs
mergehell run app.mh --union
mergehell run app.mh --random
mergehell run app.mh --manual
mergehell run app.mh --git
```

## 28.2 Check

```bash
mergehell check app.mh
```

Returns success if the file is sufficiently unresolved.

Possible failure:

```txt
fatal: no conflict markers found
hint: this appears to be valid software
```

## 28.3 AST

```bash
mergehell ast app.mh
```

Prints the conflict tree.

## 28.4 Merge

```bash
mergehell merge base.mh ours.mh theirs.mh
```

Produces canonical conflict source.

## 28.5 Format

```bash
mergehell format app.mh
```

Makes the file look more broken while preserving semantics.

## 28.6 Regret

```bash
mergehell regret app.mh
```

Explains why the program behaves the way it does.

This command is allowed to fail.

---

# 29. Formatting Rules

The official formatter is called `git blame`.

A formatter SHOULD:

* preserve conflict markers,
* preserve misleading metadata,
* align nothing,
* optionally add fake hunk headers,
* optionally add `CONFLICT (content)` banners,
* never remove emotional damage.

Example formatted output:

```txt
CONFLICT (content): Merge conflict in src/main.mh
diff --cc src/main.mh
index deadbee,c0ffee0..0000000
--- a/src/main.mh
+++ b/src/main.mh
@@@ -1,7 -1,7 +1,13 @@@ main

<<<<<<< HEAD:src/main.mh@L1-L3
...
||||||| base:src/main.mh@L1-L3
...
=======
...
>>>>>>> feature/main:src/main.mh@L1-L3
```

---

# 30. Examples

## 30.1 Hello world

```txt
<<<<<<< print
Hello, world!
=======
Goodbye, world!
>>>>>>> print
```

Run:

```bash
mergehell run hello.mh --ours
```

Output:

```txt
Hello, world!
```

## 30.2 Variable

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

## 30.3 Conditional

```txt
<<<<<<< if user.isAdmin
<<<<<<< print
Access granted
=======
Welcome maybe
>>>>>>> print
=======
<<<<<<< print
Access denied
=======
Access extremely denied
>>>>>>> print
>>>>>>> if user.isAdmin
```

## 30.4 Transaction

```txt
<<<<<<< transaction
+ chargeCard()
+ sendReceipt()
- refundCard()
- revokeReceipt()
=======
+ log("payment skipped")
>>>>>>> transaction retry/3
```

## 30.5 Full cursed module

```txt
diff --git a/stdin b/stdout
index deadbee..c0ffee0
--- a/stdin
+++ b/stdout
@@ -1,7 +1,12 @@ main

hint: prefer ours
CONFLICT (content): Merge conflict in greeting
<<<<<<< HEAD:src/main.mh@L1-L4
<<<<<<< let name
James
||||||| string default
User
=======
process.env.USER
>>>>>>> feature/env-name
=======
"Unknown"
>>>>>>> feature/greeting:src/main.mh@L1-L4

<<<<<<< print
Hello, ${name}
||||||| string
"Hello, User"
=======
Goodbye, ${name}
>>>>>>> feature/printing memoize

\ No newline at end of file
```

## 30.6 Git-generated meta-conflict

Original branch:

```txt
<<<<<<< print
Hello
=======
Goodbye
>>>>>>> print
```

Other branch:

```txt
<<<<<<< print
Hola
=======
Adios
>>>>>>> print
```

Git produces:

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

MergeHell accepts this.

The merge conflict is now a program choosing between two programs that choose between two strings.

This is considered idiomatic.

---

# 31. Compliance Levels

## 31.1 Level 0: Bad Idea

A Level 0 implementation supports:

* basic conflict hunks,
* `ours` and `theirs` strategies,
* `print`.

## 31.2 Level 1: Actually Usable Bad Idea

Supports:

* nested conflicts,
* base lanes,
* `let`,
* `if`,
* `repeat`,
* `function`,
* `call`,
* diff metadata.

## 31.3 Level 2: Git-Integrated Bad Idea

Supports:

* custom merge driver,
* custom diff driver,
* patch execution,
* Git status mode,
* Git strategy mode.

## 31.4 Level 3: Organizational Incident

Supports:

* blame strategy,
* rerere standard library,
* executable diagnostics,
* binary conflicts,
* CI integration,
* formatter that worsens files.

## 31.5 Level 4: Legal Review

Supports:

* self-modifying conflicts,
* automatic force-push,
* production deployment from unresolved merge states,
* Slack notifications saying “resolved.”

This level is not recommended.

---

# 32. Non-Goals

MergeHell does not aim to be:

* readable,
* safe,
* easy to lint,
* compatible with normal development workflows,
* emotionally supportive,
* accepted by code review.

MergeHell does aim to be:

* technically coherent,
* Git-shaped,
* deeply annoying,
* executable while appearing broken,
* able to turn real merge conflicts into language constructs.

---

# 33. Rationale for Major Design Decisions

## 33.1 Why make conflict hunks the core syntax?

Because using conflict markers only as decoration would be cowardice.

The core joke only works if the interpreter treats unresolved conflicts as meaningful structure.

## 33.2 Why support real Git conflicts?

Because the best possible feature is that Git can accidentally write valid MergeHell.

When two people edit a MergeHell file and Git produces conflict markers, the language should become more expressive, not less valid.

This makes Git a hostile macro system.

## 33.3 Why include `||||||| base`?

The base lane gives the language a third semantic layer.

Without it, each conflict is just a binary choice.

With it, a conflict can contain:

* value,
* fallback,
* type,
* default,
* previous state,
* documentation.

This turns Git’s diff3 style into a surprisingly useful syntax form.

## 33.4 Why use `>>>>>>>` metadata?

The closing marker is wasted space in normal Git conflicts.

MergeHell turns it into decorators.

Branch labels already look like meaningful metadata:

```txt
>>>>>>> feature/cache
>>>>>>> hotfix/null-user
>>>>>>> retry/3
>>>>>>> env/prod
```

So they should actually mean something.

## 33.5 Why make diff headers optional?

Diff headers are visually perfect, but requiring them would make small programs tedious.

Therefore:

* conflict hunks are required,
* diff metadata is optional,
* patch execution is advanced.

This keeps hello world short while allowing full cursed artifacts.

## 33.6 Why not require a Git repository?

Because then the language would be funny once and annoying forever.

MergeHell source should run outside Git.

Git-aware execution should be optional.

## 33.7 Why make diagnostics valid source?

Because a compiler error that can be executed is the purest expression of the language.

This allows workflows like:

```bash
mergehell build app.mh 2> errors.mh
mergehell run errors.mh --theirs
```

The build failure becomes a recovery program.

## 33.8 Why make comments unsafe?

Because in real systems, comments already lie.

MergeHell simply formalizes this.

---

# 34. Recommended Project Layout

```txt
project/
  .gitattributes
  .gitignore
  src/
    main.mh
    auth.mh
    payment.mh
  tests/
    hello.ours.expected
    hello.theirs.expected
    hello.union.expected
  regrets/
    production-incident.mh
```

Recommended `.gitattributes`:

```gitattributes
*.mh merge=mergehell
*.mh diff=mergehell
```

Recommended `.gitignore`:

```gitignore
# Do not ignore conflict files.
# They may be source code.

*.tmp
*.actually-resolved
```

---

# 35. Example README

```txt
# This repository intentionally contains unresolved merge conflicts.

Do not resolve them.

Run:

    mergehell run src/main.mh --ours

If your editor says the file is broken, the editor is working correctly.

If GitHub says this branch cannot be merged cleanly, deployment may proceed.
```

---

# 36. Final Rule

A MergeHell implementation is conforming if it follows this rule:

> When presented with unresolved conflict markers, it should not ask “how do I fix this?”
> It should ask “which side do I execute?”

A perfect MergeHell implementation goes further:

> When presented with clean code, it should fail suspiciously.

Recommended error:

```txt
fatal: no conflicts found
hint: this program appears to have been resolved by someone responsible
```
