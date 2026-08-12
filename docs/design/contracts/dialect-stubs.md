# KCS: Dialect command stubs

## Purpose

Dialect stubs let users declare command signatures for unknown dialect
extensions so the analyser stops reporting them as unresolved and reads
their arguments with the right roles, without a full registry entry.
This is essential for EDA tools (Synopsys, Cadence, Xilinx), custom
frameworks, and any Tcl extension that adds commands the LSP does not
know about.

The stub language is deliberately narrower than `CommandSpec`: it has no
subcommands, options, types, hooks, or side-effect classification, and
stub names are **not** offered in completion.  A command that needs any
of that belongs in the registry proper — see the [command registry field
reference](../compiler/command-registry.md).

## Delivery mechanisms

### External stub files

`<dialect>.tcl.stubs` files contain stub definitions, one per line.
No `#` prefix is needed.  Comments start with `#`.

```
# synopsys-eda-tcl.tcl.stubs
stub foreach_in_collection {varName:var collection body:body} -loop
stub get_cells {?-hierarchical? ?-filter? pattern:pattern} -pure
stub sizeof_collection {collection} -pure
stub expr-func sizeof 1
```

`<dialect>` is the **dialect profile name** the document is analysed under
(`tcl8.6`, `f5-irules`, `synopsys-eda-tcl`, … — the `KNOWN_DIALECTS`
vocabulary), not the name of the library being described.  A file named
after the library is never found.

The analysed file's own directory is searched first, then each ancestor
directory in turn, and the **nearest** `<dialect>.tcl.stubs` wins — a
workspace root can ship a broad bundle while a nested project overrides
it.  The sidecar is normalised into the inline grammar before parsing, so
there is exactly one parser; declarations that came from a sidecar are
flagged as such and never carry a source-positioned diagnostic, because
their spans do not belong to the analysed document.

### Inline stubs

Stub blocks are bracketed by `# tcl-lsp: stubs-begin` and
`# tcl-lsp: stubs-end` markers.  Multiple blocks per file are supported.
Stubs outside a block are ignored.  Inside a block the `tcl-lsp:` prefix
is optional, and the `stubs-begin` / `stubs-end` / `stub` keywords are
matched case-insensitively.

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

The braced argument list is **required** — a bare `stub NAME` is
rejected.  A declaration whose `:role` annotation is not one of the roles
below is dropped whole rather than being partially honoured.  Two
declarations for the same command name are last-one-wins.

### Argument roles

| Role | Meaning |
|------|---------|
| `body` | Tcl script body (recursively analysed) |
| `expr` | Expression (expr sub-language) |
| `var` | Variable name written by the command |
| `var_read` | Variable name read without modification |
| `name` | Symbolic name (proc, namespace, design name) |
| `pattern` | Pattern or regex |
| `channel` | Channel identifier |
| `value` | Generic value (default when no role given) |

Each maps onto the registry's own `ArgRole` enum at overlay-construction
time, so every downstream query is typed rather than string-keyed.

### Optional arguments

Wrap in `?...?` to mark as optional: `?-filter?`, `?count:value?`.

### Flags

Flags are recorded as a bit set on the parsed declaration.  Each names a
registry concept, but a stub records the *claim* only — it does not build
a `CommandSpec`, and **no pass currently branches on a stub flag**.  The
flags are parsed, carried on the declaration and the overlay, and exposed
for inspection; the argument roles and the declared name are what change
analysis today.

| Flag | Registry counterpart |
|------|----------------------|
| `-barrier` | `Traits::CREATES_DYNAMIC_BARRIER` |
| `-loop` | `Traits::HAS_LOOP_BODY` |
| `-pure` | `Traits::PURE` |
| `-mutator` | `SubCommand::mutator` |
| `-unsafe` | `Traits::UNSAFE` |
| `-scope_alias` | `Traits::CREATES_SCOPE_ALIAS` |

Unrecognised flag tokens are ignored rather than rejecting the stub.

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

Two shapes, in two crates.  The analyser keeps the **source-level**
record, which retains the span of the comment line for diagnostics:

```rust
pub struct StubArgDef {
    pub name: String,
    pub role: String,      // "body" / "expr" / "var" / … ; "value" by default
    pub optional: bool,
}

pub struct StubCommandDef {
    pub name: String,
    pub args: Vec<StubArgDef>,
    pub range: Span,
    pub flags: StubFlags,  // bitflags: BARRIER / LOOP / PURE / MUTATOR / UNSAFE / SCOPE_ALIAS
    pub from_sidecar: bool,
}

pub struct StubExprDef {
    pub name: String,
    pub kind: String,      // "function" or "operator"
    pub arity: u32,
    pub range: Span,
    pub from_sidecar: bool,
}
```

The registry keeps the **semantic** overlay, which drops the span and
canonicalises each role to the typed `ArgRole`:

```rust
pub struct StubArg  { pub name: String, pub role: ArgRole, pub optional: bool }
pub struct StubSig  { pub name: String, pub args: Vec<StubArg>, pub flags: StubSigFlags }
pub struct StubOverlay { /* BTreeMap<String, StubSig> */ }
```

`StubFlags` and `StubSigFlags` share a bit layout by design, so the
conversion is a straight bit copy.

Stubs are stored on the analyser's `AnalysisResult` as
`stub_commands: Vec<StubCommandDef>` and
`stub_expr_defs: Vec<StubExprDef>`, and the derived `StubOverlay` lives on
the analyser for the duration of one `analyse()` call.

### Why an overlay rather than the registry

Stubs are per-document declarations.  Merging them into the shared
`CommandRegistry` would leak one document's declarations into every other
document in the workspace and defeat the registry's caching and interning.
Consumers therefore query the registry first and the overlay second, and
the overlay is rebuilt per analysis.

`StubOverlay::fingerprint` gives a stable 64-bit hash of the overlay's
contents (order-independent, because the map is sorted).  The
compilation-unit and interprocedural-summary caches include it in their
keys, so editing a stub invalidates exactly the entries that depended on
the previous stub set.

## What stubs change

- The declared name stops drawing W123 ("unresolved command"), and it
  joins the known-command universe the unclosed-delimiter recovery
  consults.
- Argument roles feed proc-argument-trait inference, the call graph, and
  variable-usage analysis, so a `body` argument is recursively analysed.
- A stub *suppresses* the registry arity and subcommand diagnostics
  (E001 / E002 / E003) for a name that shadows a built-in — the stub is
  treated as a redefinition whose real shape the analyser cannot check.
  A stub does **not** introduce arity checking of its own.
- The LSP watches `**/*.tcl.stubs` and invalidates every tracked analysis
  when a sidecar changes, because any file may inherit its nearest
  ancestor sidecar.

What stubs do **not** change: completion offers no stub-declared name,
and no pass branches on a stub flag.

## Parsing

`scan_source_for_stubs(source)` performs a line-based pre-scan of the
source text before lexing.  It finds the `stubs-begin` / `stubs-end`
markers and parses every declaration between them, returning the command
and expression declarations separately.

`scan_sidecar_stubs(file_path, dialect)` walks from the file's directory
upwards for the nearest `<dialect>.tcl.stubs`, adapts it into the inline
grammar, and parses it with the same scanner.  `has_sidecar_stubs` is the
cheap existence probe the incremental analyser uses to decide whether it
must fall back to a full analysis.

`build_stub_overlay(defs)` converts the parsed records into the registry
overlay.

## Authoring stubs from a spec

The [command spec studio](command-spec-studio.md) renders a draft
`CommandSpec` as either an inline stubs block or a standalone
`<dialect>.tcl.stubs` file.  Because the stub language carries strictly
less than a spec, anything it cannot express is emitted as a comment
beside the stub rather than dropped silently.

## Files

| File | Purpose |
|------|---------|
| `rust/tcl-compiler/src/analyser/utils.rs` | Stub parser (inline + sidecar), `scan_source_for_stubs`, `scan_sidecar_stubs`, `scan_stub_command_names` |
| `rust/tcl-compiler/src/analyser/types.rs` | `StubCommandDef`, `StubExprDef`, `StubArgDef`, `StubFlags`, `build_stub_overlay` |
| `rust/tcl-compiler/src/analyser/state.rs` | Per-analysis overlay construction |
| `rust/tcl-compiler/src/analyser/param_traits.rs` | Overlay-aware role resolution |
| `rust/tcl-registry/src/stub_overlay.rs` | `StubOverlay`, `StubSig`, `StubArg`, `StubSigFlags` |
| `rust/tcl-spec-studio/src/render_stub.rs` | Spec → stub renderer |
| `samples/synopsys.tcl.stubs` | Example external stubs file (illustrative content; rename to `synopsys-eda-tcl.tcl.stubs` for the loader to pick it up) |
| `samples/dialect_stubs_inline_example.tcl` | Example inline stubs |
