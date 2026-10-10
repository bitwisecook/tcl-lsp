# KCS: How do I describe a C extension's commands to tcl-lsp?

> **Audience:** User
> **Type:** How-To

## Applies to

tcl-lsp CLI, claude-skill

## Question

I use a compiled Tcl extension — a `.so` or `.dll` whose `_Init` registers
commands with `Tcl_CreateObjCommand` — and tcl-lsp reports its commands as
unknown. How do I tell it they exist, and what they take, without writing every
spec by hand?

## Before you start

Nothing in a C source or a loaded package says what a native command *does*. It
can run any argument as a script, read or write any variable, create or delete
commands, set traces, return with any completion code, and be handed
attacker-controlled input, so tcl-lsp describes every extension command at the
most conservative reading of all: unknown arity, a barrier to optimisation and
renaming, unknown reads and writes, a taint sink and source, hidden in a safe
interpreter, never pure. The import tools below give you the commands and what the
source *states* of them — the arity a usage message gives, the subcommands an
option table names, the package it provides — as proposals with their evidence.
They never narrow a command, because nothing they read proves anything about what
it does.

## Answer

1. **Scan the C source.** Point `tcl spec import` at the directory holding the
   extension's `.c` and `.h` files:

   ```sh
   tcl spec import --c-source src/ --out myext.tclspec
   ```

   It reads `Tcl_CreateObjCommand`, `Tcl_CreateCommand` and
   `Tcl_NRCreateCommand` registrations, `Tcl_PkgProvide`, the `Tcl_WrongNumArgs`
   usage message of each command's own procedure, and the `Tcl_GetIndexFromObj`
   table it reads for subcommands. `--c-source` may be repeated. Every row has
   the provenance `c-scan` and the file and line each fact was read at.

   One pack describes one extension, the commands one entry point registers. If
   the directory holds more than one (a `Foo_Init` in one file and a `Bar_Init`
   in another), the import lists them and stops; name the one to describe with
   `--entry Foo`, and the pack has the registrations in the functions `Foo_Init`
   reaches by name, whatever file they are in. A registration in a function no
   entry point names is left out, with a warning saying where it is.
2. **Or, or as well, probe a real shell.** `tcl spec import --probe PACKAGE`
   requires the package in a `tclsh` and lists the commands it added to any
   namespace; `--tclsh PATH` names the shell. That runs the package's own code, so
   it runs only for a package your project's policy has opted in, as `tcl spec
   test` does: set `[build] allow-build-scripts = true` in `tclpkg.toml` and run
   `tcl pkg trust PACKAGE`. Rows it finds carry the provenance `probe`, and a
   command both the scan and the probe found carries both.
3. **Read the header before the commands.** It lists each command with its
   provenance and evidence, and three things the scan could not turn into a
   command: a registration whose name is computed (a factory, a table indexed by
   a loop) is listed as *computed* and has to be declared by hand; a call to the
   `TclOO` C API or a C-built ensemble registers commands the scan cannot read;
   and what the procedure's own body calls — a script or expression evaluation,
   a variable by name, the command table. Only the registered procedure's own body
   is read, so the absence of such a call says nothing about its callees.
4. **Narrow what you know.** In the pack, state the facts you can vouch for
   (`arity`, `traits`, `side_effect` rows) the way any pack does ([how to write a
   SpecTcl pack](kcs-howto-write-a-tclspec-pack.md)). For a command you only want
   known in one document, a stub does the same: `# tcl-lsp: stub NAME {ARGS}
   -extension` declares it at the same default, and `-pure` or `-mutator` narrows
   its effects (see [how to annotate commands with
   stubs](kcs-howto-annotate-commands-with-stubs.md)).
5. **Validate** the pack the way any other is, with `mcp__tcl-lsp__spectcl_check`.

## How to tell it worked

The summary on stderr counts the commands described, how many came from each
source, the computed registrations and the calls the scan could not read. A
command you can call from a script now draws no unknown-command diagnostic, and
the checks that depend on what it does — an unused result, a hoisted constant, a
renamed local around it — stay conservative until you narrow it.

## Related

- [How to annotate commands with stubs](kcs-howto-annotate-commands-with-stubs.md)
- [How to tell the server a binary extension loads another package](kcs-howto-declare-a-package-a-binary-extension-loads.md)
- [How to derive version ranges from releases](kcs-howto-derive-version-ranges-from-releases.md)
- [C extension shim](../design/runtime/c-extension-shim.md)
- [KCS index](README.md)
- [Glossary](../GLOSSARY.md)
