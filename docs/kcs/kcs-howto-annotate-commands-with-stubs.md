# KCS: How do I annotate an external Tcl command with a stub?

> **Audience:** User
> **Type:** How-To

## Applies to

VS Code, Zed, JetBrains, Neovim, Helix, Emacs, Sublime Text, tcl-lsp CLI

## Question

How do I tell tcl-lsp about a command from a third-party Tcl library — one
whose source the analyser cannot see — so that arity checks, the call graph,
and trait inference treat its arguments correctly?

## Before you start

- The command lives outside the workspace (a shared library, a C extension,
  an `sqlite3`-style instance command, an EDA-vendor builtin).
- You know the command's argument shape and which arguments are scripts,
  expressions, or variable names.
- You have write access to the file that calls the command, or to a
  workspace-wide `<dialect>.tcl.stubs` file.

## Answer

Stubs are declared either inline in the Tcl file that uses the command, or
in a sidecar `<dialect>.tcl.stubs` file. Both forms use the same syntax. The
analyser merges the declared signature into the active command registry for
the duration of analysis, so the call graph, arity checker, and proc-arg
trait inferencer all see the stubbed command as if it were a built-in.

### Inline stubs (one file)

Wrap the declarations in a `# tcl-lsp: stubs-begin` / `# tcl-lsp: stubs-end`
block. Stubs outside the markers are ignored.

```tcl
# tcl-lsp: stubs-begin
# tcl-lsp: stub db_eval {sql script:body} -barrier
# tcl-lsp: stubs-end

proc on_row {} { ... }

proc main {} {
    db_eval "SELECT name FROM t" {on_row}
}
```

After this, `tcl callgraph` reports `::main → ::on_row`, the `script`
argument is recognised as a Tcl script, and arity violations on `db_eval`
are reported.

### Sidecar stubs file (whole workspace)

Drop a `<dialect>.tcl.stubs` file next to your Tcl sources — for example
`sqlite.tcl.stubs` or `synopsys.tcl.stubs`. Each non-comment line is one
declaration:

```
stub db_eval {sql script:body} -barrier
stub sqlite3 {name path}
stub redirect {?-file? filename body:body}
```

The `#` prefix and the `tcl-lsp:` tag are optional in this form.

### Stub syntax

```
stub <command-name> ?<subcommand>? {arg1:role arg2 ?optArg:role?} ?flags...?
```

A subcommand word between the command name and the braced argument list
turns the stub into an ensemble entry — multiple stubs with the same
command name but different subcommands fold into a single dispatch
table so `db eval` and `db transaction` can declare different shapes.

Argument roles (after the `:`):

| Role       | Meaning                                                |
|------------|--------------------------------------------------------|
| `body`     | Tcl script body — recursively analysed                 |
| `expr`     | Expression (expr sub-language)                         |
| `var`      | Variable name written by the command                   |
| `var_read` | Variable name read without modification                |
| `name`     | Symbolic name (proc name, namespace name)              |
| `pattern`  | Pattern or regex                                       |
| `channel`  | Channel identifier                                     |
| `value`    | Generic value (default when no role is specified)      |

An argument wrapped in `?...?` is optional. An argument literally named
`args` marks the tail as variadic.

Flags:

| Flag           | What it does today                                  |
|----------------|-----------------------------------------------------|
| `-barrier`     | Marks the command as crossing an interpreter boundary; downstream passes recognise it via `StubCommandDef.barrier`. |
| `-loop`        | Records `StubCommandDef.loop`; informational, kept for future code-flow specialisation. |
| `-pure`        | Recorded on the `StubCommandDef`. *Not yet* fed into purity / constant-folding propagation. |
| `-mutator`     | Recorded on the `StubCommandDef`. *Not yet* fed into side-effect classification. |
| `-unsafe`      | Recorded on the `StubCommandDef`. *Not yet* fed into safe-interp checks. |
| `-scope_alias` | Recorded on the `StubCommandDef`. *Not yet* fed into upvar-style scope tracking. |

The flags marked *not yet* fed through are parsed and surfaced on the
analyser's `AnalysisResult.stub_commands` for inspection, but the
purity / side-effect / safe-interp passes do not currently change
behaviour based on them. Argument roles (`body`, `expr`, `var`,
`var_read`, `pattern`, `channel`, `name`) and arity *do* drive
analyses today.

### Worked example — sqlite `db eval` / `db transaction`

The sqlite3 Tcl extension creates an instance command (`sqlite3 db
:memory:` → command `db`) whose subcommands include `eval`,
`transaction`, `function`, `onecolumn`, and so on. Each subcommand has
its own shape:

```tcl
db eval $sql ?$rowvar? ?$script?     ;# row callback at the trailing slot
db transaction ?$type? $script       ;# script always trailing
db function NAME ?-argcount N? $script
```

Stub each subcommand separately. The `?rowvar?` optional slot is
honoured by the resolver — the body's index shifts based on the actual
call:

```tcl
# tcl-lsp: stubs-begin
# tcl-lsp: stub db eval {sql ?rowvar? script:body} -barrier
# tcl-lsp: stub db transaction {script:body} -barrier
# tcl-lsp: stub db function {name script:body} -barrier
# tcl-lsp: stubs-end

package require sqlite3
sqlite3 db :memory:

proc handle_row {} { puts "$name = $value" }
proc apply_change {} {}

proc dump {} {
    db eval {SELECT name, value FROM kv} {handle_row}     ;# body at arg 2
}

proc dump_rowvar {} {
    db eval {SELECT name FROM kv} row {handle_row}        ;# body at arg 3
}

proc run_in_tx {} {
    db transaction {apply_change}                          ;# body at arg 1
}
```

With these stubs in place, `tcl callgraph` reports `::dump →
::handle_row`, `::dump_rowvar → ::handle_row`, and `::run_in_tx →
::apply_change`. The unused-proc analyser leaves the callbacks alone.

This stub block has been exercised against `libsqlite3-tcl` 3.45.1 — the
exact source runs in `tclsh` and our analyser sees every callback.

### Expression-function and operator stubs

Custom math functions or infix operators in the `expr` sub-language are
stubbed separately:

```tcl
# tcl-lsp: stubs-begin
# tcl-lsp: stub expr-func sizeof 1
# tcl-lsp: stub expr-op  starts_with 2
# tcl-lsp: stubs-end
```

The numeric argument is the arity — 1 for unary functions, 2 for binary
operators, and so on.

## How to tell it worked

- Run `tcl callgraph <file>` and check that the callback procs declared in
  the stubbed command's body argument appear as outgoing edges from the
  caller.
- Open the file in your editor and confirm that the stubbed command no
  longer raises "unknown command" / "unresolved command" diagnostics.
- Pass too few or too many arguments — the arity check should report it.

## Related

- [KCS: What annotations does tcl-lsp understand?](kcs-qa-tcl-lsp-annotations.md)
- [How to suppress diagnostics inline](kcs-howto-suppress-diagnostics.md)
- [KCS index](README.md)
- [Glossary](../GLOSSARY.md)
