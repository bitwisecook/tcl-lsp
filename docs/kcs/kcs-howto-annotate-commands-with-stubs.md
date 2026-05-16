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
stub <command-name> {arg1:role arg2 ?optArg:role?} ?flags...?
```

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

| Flag           | Meaning                                          |
|----------------|--------------------------------------------------|
| `-barrier`     | Command creates a dynamic analysis barrier       |
| `-loop`        | Command has a loop body                          |
| `-pure`        | Command is side-effect-free                      |
| `-mutator`     | Command mutates state                            |
| `-unsafe`      | Command is unsafe                                |
| `-scope_alias` | Command creates a scope alias (like `upvar`)     |

### Worked example — sqlite `db eval`

`sqlite3 db_eval` (and the instance-command equivalent `db1 eval ...`) takes
a SQL string followed by an optional callback script that runs for each
row. Stub it as:

```tcl
# tcl-lsp: stubs-begin
# tcl-lsp: stub db1 {subcommand args}
# tcl-lsp: stub sqlite_eval {sql script:body} -barrier
# tcl-lsp: stubs-end

proc handle_row {} { puts "row: $name=$value" }

proc dump {} {
    sqlite_eval "SELECT name, value FROM kv" {handle_row}
}
```

With the stub in place, `tcl callgraph` shows the edge `::dump →
::handle_row`, and the unused-proc analyser does not flag `handle_row` as
dead code.

For commands that read or write variables in the caller's frame, mark the
argument with `var` or `var_read`:

```tcl
# tcl-lsp: stub sqlite_eval {sql script:body rowvar:var} -barrier
```

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
