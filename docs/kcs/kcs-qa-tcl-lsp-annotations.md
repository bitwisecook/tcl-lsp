# KCS: What annotations does tcl-lsp understand in source files?

> **Audience:** User
> **Type:** Q&A

## Applies to

VS Code, Zed, JetBrains, Neovim, Helix, Emacs, Sublime Text, tcl-lsp CLI

## Question

What `# tcl-lsp:` / `# noqa` comments does the analyser recognise, and what
does each one do?

## Answer

tcl-lsp reads five kinds of structured comment from your Tcl source.
They all live in plain Tcl comments — no separate config file is required,
and they have no effect on the running interpreter.

### 1. File-wide diagnostic suppression

`# tcl-lsp: disable=CODE1,CODE2` at the top of a file turns off the listed
diagnostic codes for the entire file. `=*` disables everything. The
analyser scans only the leading comment / blank lines, so the directive
must appear before the first command.

```tcl
# tcl-lsp: disable=W210,W123

proc demo {} {
    # W210 (read-before-set) and W123 will not fire in this file.
}
```

For per-line and inline suppression, see
[KCS: How do I suppress diagnostics?](kcs-howto-suppress-diagnostics.md).

### 2. Per-line suppression (`# noqa`)

A `# noqa` comment on line *N* suppresses every diagnostic on line *N+1*.
`# noqa: CODE` narrows it to specific codes (comma-separated).

```tcl
# noqa: W210
set x [other_var]
```

This is intended for the cases where a directive needs to sit immediately
above the offending command — for example, on a brace-tail line or before
another comment that itself triggers a diagnostic. For inline-on-the-same-
line suppression and project-wide rules, follow
[kcs-howto-suppress-diagnostics.md](kcs-howto-suppress-diagnostics.md).

### 3. Command stubs

Stubs declare the signature of a command the analyser cannot see —
typically a third-party library, an instance command created by a factory
(like `sqlite3 db1`), or a vendor builtin (EDA tools, F5 iApps). They go
inside a `# tcl-lsp: stubs-begin` / `# tcl-lsp: stubs-end` block. Stubs
outside the markers are ignored.

```tcl
# tcl-lsp: stubs-begin
# tcl-lsp: stub db_eval {sql script:body} -barrier
# tcl-lsp: stub redirect {?-file? filename body:body}
# tcl-lsp: stubs-end
```

The same syntax works in a workspace-wide `<dialect>.tcl.stubs` file, with
the `# tcl-lsp:` prefix optional.

Argument roles include `body` (recursively analysed script), `expr`, `var`,
`var_read`, `name`, `pattern`, `channel`, and the default `value`. Roles
plus arity drive the analyses today — adding a `body` role makes the call
graph descend into the script, adding `var` lets the variable-usage
analyser see the write, and so on.

Flags (`-barrier`, `-loop`, `-pure`, `-mutator`, `-unsafe`, `-scope_alias`)
are parsed and recorded on the `StubCommandDef`, but most of them are
not yet wired into downstream passes — only `-barrier` is consulted by
the call-graph scanner. See
[kcs-howto-annotate-commands-with-stubs.md](kcs-howto-annotate-commands-with-stubs.md)
for which flags affect analysis today.

**Worked example — sqlite `eval`:** sqlite's per-row callback is the
trailing argument, which is a Tcl script. Stubbing it as `body` lets the
call graph see edges into the callback procs:

```tcl
# tcl-lsp: stubs-begin
# tcl-lsp: stub sqlite_eval {sql script:body} -barrier
# tcl-lsp: stubs-end

proc on_row {} { ... }

proc dump {} {
    sqlite_eval "SELECT * FROM kv" {on_row}
}
```

After this, `tcl callgraph` reports the edge `::dump → ::on_row` and the
unused-proc analyser leaves `on_row` alone.

Full syntax and more examples live in
[kcs-howto-annotate-commands-with-stubs.md](kcs-howto-annotate-commands-with-stubs.md).

### 4. Expression-function and operator stubs

Custom math functions or infix operators in the `expr` sub-language are
declared inside the same stub block:

```tcl
# tcl-lsp: stubs-begin
# tcl-lsp: stub expr-func sizeof 1
# tcl-lsp: stub expr-op  starts_with 2
# tcl-lsp: stubs-end
```

The trailing number is the arity (1 for unary functions, 2 for binary
operators, and so on).

### 5. Packages another package loads

`# tcl-lsp: package NAME provides PKG …` says that loading `NAME` also
loads the packages listed. It exists for a compiled extension — a `.dll` /
`.so` whose C `Init` calls `Tcl_PkgRequire` or `Tk_InitStubs` — which
loads a package with nothing in any Tcl source to say so, so nothing the
analyser can read would ever reveal it.

```tcl
# tcl-lsp: package myExtension provides Tk
package require myExtension

ttk::frame .f
pack .f
```

Tk's completions, hover and checks now switch on, and the "requires
`package require Tk`" warning (W120) goes quiet — exactly as if the file
had said `package require Tk` alongside the extension. Name several
packages on one line, and put the comment anywhere in the file: it names
the extension, so it does not depend on where it sits, and it does nothing
in a file that never requires that extension.

The same fact can be declared once for a whole project instead, under
`[packages.provides]` in `.tcl-lsp.ini` — see
[how do I tell tcl-lsp that a binary extension also loads Tk?](kcs-howto-declare-a-package-a-binary-extension-loads.md).

### What annotations do *not* do

- They do not change runtime Tcl behaviour — they are plain `#` comments.
- They are not honoured by `eval`-style commands at run time; they only
  inform the analyser.
- They are scoped to the file they appear in (with the exception of
  `.tcl.stubs` files, which apply workspace-wide).

## Related

- [How to annotate an external Tcl command with a stub](kcs-howto-annotate-commands-with-stubs.md)
- [How do I tell tcl-lsp that a binary extension also loads Tk?](kcs-howto-declare-a-package-a-binary-extension-loads.md)
- [How to suppress diagnostics inline](kcs-howto-suppress-diagnostics.md)
- [KCS index](README.md)
- [Glossary](../GLOSSARY.md)
