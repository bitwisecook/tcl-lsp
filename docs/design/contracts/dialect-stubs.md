# Dialect command stubs

## Purpose

Dialect stubs let users declare command signatures for unknown dialect
extensions so the LSP provides completion, diagnostics, and semantic
understanding without a full registry entry.  This is essential for
EDA tools (Synopsys, Cadence, Xilinx), custom frameworks, and any Tcl
extension that adds commands the LSP does not know about.

## Delivery mechanisms

### Workspace sidecar files

A `<dialect>.tcl.stubs` file contains stub definitions, one per line, with no
`#` prefix; `#` starts a comment. `scan_sidecar_stubs` walks up from the
analysed document to the nearest such file for the active dialect.

Sidecar declarations participate in resolution exactly like inline ones, but
are flagged `from_sidecar` — their spans are synthetic, so they can never
produce a source-positioned shadow diagnostic in the document being analysed.
The incremental analyser also gates on sidecar readability, because a sidecar
signature affects every document under it.

```
# synopsys.tcl.stubs
stub foreach_in_collection {varName:var collection body:body} -loop
stub get_cells {?-hierarchical? ?-filter? pattern:pattern} -pure
stub sizeof_collection {collection} -pure
stub expr-func sizeof 1
```

### Inline stubs

Stub blocks are bracketed by `# tcl-lsp: stubs-begin` and
`# tcl-lsp: stubs-end` markers.  Multiple blocks per file are supported.
Stubs outside a block are ignored.

```tcl
# tcl-lsp: stubs-begin
# tcl-lsp: stub foreach_in_collection {varName:var collection body:body} -loop
# tcl-lsp: stub get_cells {pattern:pattern} -pure
# tcl-lsp: stub expr-func sizeof 1
# tcl-lsp: stub expr-op contains 2
# tcl-lsp: stubs-end
```

## Command stub syntax

```
stub <command-name> {arg1:role arg2 ?optArg:role?} ?flags...?
```

### Argument roles

Each role word maps to one registry `ArgRole` through
`tcl_registry::model::role_for_word`. An unrecognised word is **not** an error
— it falls through to `Value`, so a typo silently degrades to the generic role
rather than rejecting the stub.

| Role | `ArgRole` | Meaning |
|------|---|---------|
| `body` | `Body` | Tcl script body (recursively analysed) |
| `expr` | `Expr` | Expression (expr sub-language) |
| `var` | `VarWrite` | Variable name written by the command |
| `var_read` | `VarRead` | Variable name read without modification |
| `name` | `Name` | Symbolic name (proc, namespace, design name) |
| `pattern` | `Pattern` | Pattern or regex |
| `channel` | `Channel` | Channel identifier |
| `command_prefix` | `CommandPrefix` | A command prefix invoked as a callback |
| `value` | `Value` | Generic value — the default, and the fallback for any unknown word |

### Optional arguments

Wrap in `?...?` to mark as optional: `?-filter?`, `?count:value?`.

### Flags

The trailing flag set is parsed into the analyser-side `StubFlags` bitflags
(`analyser/types.rs`). It is recorded and never read: no consumer has ever
asked a stub for a flag, so gap ruling R1's ingestion deliberately does not
carry the set onto the declaration — principle P-C, a fact comes back with the
consumer that needs it. The recognised words are:

| Flag | Meaning |
|---|---|
| `-barrier` | creates a dynamic barrier |
| `-loop` | has a loop body |
| `-pure` | no side effects |
| `-mutator` | mutates its target |
| `-unsafe` | unsafe in a safe interpreter |
| `-scope_alias` | creates a scope alias |

## Expression stubs

Custom math functions and infix operators for dialects that extend expr:

```
stub expr-func <name> ?arity?    # default arity 1
stub expr-op <name> ?arity?      # default arity 2
```

Examples:
```
stub expr-func sizeof 1
stub expr-func clamp 3
stub expr-op contains 2
stub expr-op starts_with 2
```

## Data model

`StubCommandDef` / `StubArgDef` / `StubExprDef`
(`rust/tcl-compiler/src/analyser/types.rs`) are the analyser-side records:
name, parsed argument list, the span of the declaring comment line, a
`StubFlags` bitflag set, and the `from_sidecar` marker. They are collected onto
`AnalysisResult` and keep their spans so diagnostics can point at the
declaration.

## Stubs are declarations (gap ruling R1)

A stub is a **per-document** declaration, so it must not pollute the
`CommandRegistry` that every document in a workspace shares — mutating the
shared registry per analysis call would also defeat the interning and caching
the registry relies on. It is nevertheless the *same kind of fact* the
catalogue states, so it ingests through the same pipeline rather than into a
parallel one:

- `StubCommandDef::to_declared_command` produces a
  `tcl_registry::model::DeclaredCommand` — a name, registry `ArgRole`
  arguments, and an ordinary `SurfaceDeclaration` whose provider is
  `Provider::Document`, whose applicability is the whole
  `VersionAxisId::document()` axis, and whose predicate is `None`.
- The declaration carries its **provenance**: `Provenance::Document` for an
  inline block, `Provenance::WorkspaceUntrusted` for a `.tcl.stubs` sidecar —
  §6.4's two lowest trust classes.
- `build_declared_surface` collects them into the document's
  `DeclaredSurface`, rebuilt on each `analyse()` call and held on the
  (single-threaded) analyser.

`tcl_registry::model::DocumentCommandSurface` is **the** door onto the command
surface one document analyses against: the catalogue generation plus that
document's own declarations, asked once. No consumer holds a registry and a
second table and unions the answers itself.

Two properties are load-bearing:

- **Roles are typed at ingestion.** The source string (`"body"`, `"var"`, …)
  is canonicalised to `ArgRole` through `role_for_word` once, so every
  subsequent query is typed and no consumer re-parses a role word.
- **A declaration widens, never narrows.** `DocumentCommandSurface`'s role
  lookup unions the catalogue's answer with the document's, which is §6.4's
  untrusted-tier rule read literally: a declaration may improve assistance and
  can never weaken a shipped analysis fact.

The declared surface is what feeds parameter-trait inference, role lookup, and
command-resolution for stubbed commands. Cache invalidation rides the ordinary
inputs — the document's own text for an inline block, and lsp-db's
`sidecar_stubs_epoch` salsa input for a sidecar — not a bespoke fingerprint.

## Parsing

`tcl_compiler::analyser::utils::scan_source_for_stubs(source)` is a line-based
pre-scan run before lexing: it finds the begin/end markers and parses every
stub definition between them. `scan_sidecar_stubs` does the same for the
nearest `<dialect>.tcl.stubs` ancestor file.

## Stub generation

The spec studio renders a `CommandSpec` back out as a stub line
(`rust/tcl-spec-studio/src/render_stub.rs`), in either the inline
`# tcl-lsp: stubs-begin` form or a standalone sidecar file. The stub language
is narrower than a full spec, so **what a stub cannot carry is emitted as a
comment beside it** rather than dropped — see
[command-spec-studio.md](command-spec-studio.md). Roles map through the
inverse of `role_for_word`, so a rendered stub parses back to the roles the
draft declared.

## Key files

| File | Purpose |
|---|---|
| `rust/tcl-compiler/src/analyser/utils.rs` | `scan_source_for_stubs`, `scan_sidecar_stubs` |
| `rust/tcl-compiler/src/analyser/types.rs` | `StubCommandDef`, `StubArgDef`, `StubExprDef`, `StubFlags` |
| `rust/tcl-registry/src/model/declaration.rs` | `DeclaredCommand`, `DeclaredArgument`, `DeclaredSurface`, `DocumentCommandSurface`, `role_for_word` |
| `rust/tcl-spec-studio/src/render_stub.rs` | stub rendering |
| `samples/` | example sidecar and inline stub files |
