# KCS: How do I annotate an external Tcl command with a stub?

> **Audience:** User
> **Type:** How-To

## Applies to

VS Code, Zed, JetBrains, Neovim, Helix, Emacs, Sublime Text, tcl-lsp CLI

## Question

How do I tell tcl-lsp about a command from a third-party Tcl library — one
whose source the analyser cannot see — so that the call graph and trait
inference treat its arguments correctly?

## Before you start

- The command lives outside the workspace (a shared library, a C extension,
  an EDA-vendor builtin).
- You know the command's argument shape and which arguments are scripts,
  expressions, or variable names.
- You have write access to the file that calls the command, or to a
  workspace-wide stubs file.

## Before you start

Stubs are the quick, legacy fallback: no subcommands, no arity checking,
just enough for the analyser to stop calling a command unknown. For the
full treatment — hover, options, subcommands, version gates — write a
[SpecTcl pack](kcs-howto-write-a-tclspec-pack.md) instead. A pack's `arg
-role` values are almost the same words as a stub's roles below (`body` →
`Body`, `var` → `VarWrite`, `var_read` → `VarRead`, and so on), so a stub
you already have is a quick starting point for one.

## Answer

Stubs are declared either inline in the Tcl file that uses the command, or
in a sidecar stubs file. Both forms use the same syntax. The analyser merges
the declared signature into the command surface for the duration of
analysis, so the call graph and the argument-role inferencer see the stubbed
command as if it were built in.

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

After this, `tcl callgraph` reports `::main → ::on_row`, and the `script`
argument is recognised as a Tcl script rather than an opaque string.

### Sidecar stubs file (whole workspace)

A sidecar is named after the **dialect**, not after the library:
`<dialect>.tcl.stubs`, where `<dialect>` is the dialect profile the file is
analysed under — `tcl8.6.tcl.stubs`, `f5-irules.tcl.stubs`,
`synopsys-eda-tcl.tcl.stubs`, and so on. A file named after the library
(`sqlite.tcl.stubs`) is never loaded.

tcl-lsp looks in the analysed file's own directory first, then each parent
directory, and uses the **nearest** match. Put a broad bundle at the
workspace root, and override it closer to a subproject if you need to.

Each non-comment line is one declaration; the `#` prefix and the `tcl-lsp:`
tag are not used in this form:

```
stub db_eval {sql script:body} -barrier
stub sqlite3 {name path}
stub redirect {?-file? filename body:body}
```

A declaration in the file being analysed beats a sidecar declaration of the
same name.

### Stub syntax

```
stub <command-name> {arg1:role arg2 ?optArg:role?} ?flags...?
```

The braced argument list is required — `stub NAME` on its own is ignored.
Declaring the same command twice keeps the last declaration.

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

An argument wrapped in `?...?` is optional. A role that is not in this table
throws the whole declaration away, so check your spelling if a stub seems to
have no effect.

The flags `-barrier`, `-loop`, `-pure`, `-mutator`, `-unsafe`, and
`-scope_alias` are accepted and recorded against the command, but no
analysis currently changes its answer because of one. Argument roles are
what do the work today.

### What a stub does not do

- **It does not add ensembles.** There is no subcommand form: `stub db eval
  {...}` is not a valid declaration and is silently ignored. Give each
  entry point its own top-level name, or leave the ensemble unstubbed.
- **It does not turn on arity checking.** A stub declares a name so the
  analyser stops calling it unknown; it does not make tcl-lsp count the
  arguments you pass. Where a stub shadows a built-in command, it
  *suppresses* the built-in arity and subcommand checks instead.
- **It does not carry types, options, or side effects.** Those need a real
  registry entry — see
  [how to add a library to the command registry](kcs-howto-add-command-registry-package.md).

### Expression-function and operator stubs

Custom math functions or infix operators in the `expr` sub-language are
stubbed separately:

```tcl
# tcl-lsp: stubs-begin
# tcl-lsp: stub expr-func sizeof 1
# tcl-lsp: stub expr-op  starts_with 2
# tcl-lsp: stubs-end
```

The numeric argument is the arity. It defaults to 1 for a function and 2 for
an operator when you leave it out.

## How to tell it worked

- Run `tcl callgraph <file>` and check that the callback procs declared in
  the stubbed command's body argument appear as outgoing edges from the
  caller.
- Open the file in your editor and confirm that the stubbed command no
  longer raises the "unresolved command" hint.
- For a sidecar, confirm the filename matches the dialect the file is
  analysed under — a mismatch is the most common reason a sidecar appears to
  be ignored.

## Related

- [How to write a SpecTcl pack](kcs-howto-write-a-tclspec-pack.md) — the
  fuller replacement for a stub, once you need subcommands or options.
- [How to add a third-party Tcl library to the command registry](kcs-howto-add-command-registry-package.md)
- [KCS: What annotations does tcl-lsp understand?](kcs-qa-tcl-lsp-annotations.md)
- [How to suppress diagnostics inline](kcs-howto-suppress-diagnostics.md)
- [KCS index](README.md)
- [Glossary](../GLOSSARY.md)
