# KCS: How are Tcl command names resolved across namespaces?

> **Audience:** Contributor
> **Type:** Q&A

## Applies to

all-editors, tcl-lsp-cli, analyser

## Question

When a script calls a command by a bare name (`helper`), a relative
qualified name (`inner::p`), or an absolute name (`::inner::p`), which
definition does tcl-lsp decide it dispatches to?

## Answer

Command lookup uses the namespace and command table at the actual invocation.
For a call made from namespace `ns`:

1. An absolute name beginning with `::` selects that qualified command.
2. A relative name tries `ns`, the supported `namespace path` entries in
   order, and then the global namespace. The first existing command wins;
   a namespace merely existing does not count. `namespace path` is
   available in C Tcl8.5 and later.
3. There is no ancestor walk: `helper` inside `::a::b` does not reach
   `::a::helper` unless the lookup path includes `::a`.
4. A procedure body resolves a command when it executes. A definition
   later in the file helps only if it has executed before that call.

A relative namespace path entry names a child of the current namespace.
The path setter errors if that namespace does not exist; namespace lookup
has no command-style global fallback.

The shared naming owner supplies lookup order. The source execution owner
supplies which commands exist at that point, including imports, aliases,
renames, package loaders, and earlier argument substitutions. Native compiler
choices are retained separately: some Tcl releases compile particular commands
before execution, so a spelling alone cannot prove the operation that runs.

If a dynamic path, provider, callback, or selected frame cannot be resolved,
the result keeps that uncertainty. It does not replace an unknown path with
an empty one or search unrelated namespaces for a matching command tail.
The virtual machine and standalone runtime use their actual command tables.

Contributors should start with the
[command-resolution contract](../design/contracts/command-resolution.md) and
[implementer guide](../design/compiler/name-resolution-implementer-guide.md).
