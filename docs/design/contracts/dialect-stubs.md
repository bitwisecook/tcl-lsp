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

`tcl_registry::model::role_for_word_checked` is the role vocabulary, and the
directive parser is one of its callers: a word it does not know is a typo, so
the whole declaration is dropped and the command stays unresolved rather than
half-declaring with a silently generic role. `role_for_word` is the same
lookup with the "value is the default" fallback an argument written without a
`:role` annotation gets. A second list of accepted words beside it is how a
role gets documented but stays unusable.

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
(`analyser/types.rs`). Each flag is a declared behavioural fact about the
command, and it lands on the field its catalogue counterpart uses:
`StubCommandDef::declared_traits` and `declared_side_effects` put it on
`DeclaredCommand::traits` and `DeclaredCommand::side_effects`, and
`DocumentCommandSurface::traits`, `invocation_traits` and `side_effects`
answer it, so a stubbed command reads the way a catalogued one does to every
consumer that asks the surface. The recognised words are:

| Flag | Meaning | Catalogue field | What reads it |
|---|---|---|---|
| `-barrier` | creates a dynamic barrier | `Traits::CREATES_DYNAMIC_BARRIER` | the minifier's rename barriers (`find_rename_barriers`): the scope the command runs in keeps its local names, as around `vwait` |
| `-loop` | has a loop body | `Traits::HAS_LOOP_BODY` | the loop-termination checks (`bounds_checks::loop_shape`): with an `expr` condition and a `body` word, a constant-false condition is `W240` and a constant-true one whose body never leaves the loop is `W241`, as for `while` |
| `-pure` | no side effects | `Traits::PURE` | side-effect classification (`side_effects::classify_side_effects_in`): the call is pure, so a procedure that only calls it is pure in the interprocedural summary and the unused result of calling that procedure can go (`O126`) |
| `-mutator` | reads and rewrites its target | `Traits::READS_BEFORE_WRITE`, and a declared `SideEffect` reading and writing `SideEffectTarget::Variable` | lowering reads the target before the write, as it does for `lset` and `lappend`, so the store feeding it stays live (no `O109`); side-effect classification states the variable effect instead of the unknown write |
| `-unsafe` | unsafe in a safe interpreter | `Traits::UNSAFE` with `Traits::SAFE_INTERP_HIDDEN` | the safe-interpreter gate: a call inside a safe interpreter's evaluation body is `W129`, as `exec` is |
| `-scope_alias` | creates a scope alias | `Traits::CREATES_SCOPE_ALIAS` | the call-site scan (`unit_scope::note_surface_var_writes`): every name the command takes is bound to a cell another body may write, so a later `$name` dispatch is not read as a known literal and the parameter fold is withheld (`I230`), as for `upvar`; the minifier leaves global names alone |

A stub with no flags states no behaviour, and side-effect classification
treats it exactly as an undeclared command: an unknown read and write, never
pure.

Three consumers still read these facts off the catalogue alone, so a stub's
flags do not reach them yet: the optimiser's own elimination gate and GVN (a
`-pure` call's unused result goes only through the interprocedural summary,
so `set a [mypure $x]` written directly is kept), SSA's barrier-def walk
(which reads no declared role either), and memory SSA's clobber verdict
(which treats every command the catalogue lacks as clobbering, flags or
not — the conservative answer).

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

## Stubs are declarations

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
  a label for explanation, binding selection, and invalidation, not a
  precision class.
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
- **Roles resolve against the call, not the declaration text.** A declaration
  is a positional shape with optional slots, so `DeclaredCommand::arg_indices_for_role`
  takes the number of words the call supplies and fills optional slots left
  to right: `{?table? row:var}` invoked as `fetch out` writes index 0, and a
  call with fewer words than the declaration requires maps to nothing.
- **A declaration answers where it speaks — nearest-wins.** A stub is a
  workspace-authored fact, so it is an input to analysis on the same footing
  as a shipped spec
  ([value-transfers.md](../compiler/value-transfers.md) § *Rulings*,
  ruling 3): the document's own declaration answers for the command it
  declares, and the catalogue answers everywhere else. `security_floor`'s
  monotone merge (invariant I6) still holds over it, because that floor is a
  security contract rather than a precision cap.
  `DocumentCommandSurface` answers nearest-wins: `arg_indices_for_role`,
  `command_prefixes`, `traits`, `invocation_traits` and `side_effects` read
  the declaration alone for a name the document declares, so a stub that
  redeclares `after {ms script}` states that its second word is a value and
  the catalogue's `Body` role is not assigned. `traits` and `side_effects`
  keep a redeclared shipped command's security traits and side effects
  beneath the declaration's own — the floor `SecurityFloor::apply` holds a
  pack override to — so a stub cannot take `exec`'s `UNSAFE` away.

The declared surface is what feeds parameter-trait inference, role lookup, and
command-resolution for stubbed commands. Cache invalidation rides the ordinary
inputs — the document's own text for an inline block, and lsp-db's
`sidecar_stubs_epoch` salsa input for a sidecar — not a bespoke fingerprint.

## Every role consumer reads the declared surface

A declared role means what a `CommandSpec::arg_roles` row means, so the
consumers of that row read the document's surface too, not the bare
catalogue. `UnitBuildOptions::declared_commands` carries the surface into a
build and `CompilationUnit::declared_commands` owns a copy, so a pass that
runs after the build — `with_interprocedural` above all — asks what the
lowering asked.

- **Lowering** resolves a generic call's `Body` / `LambdaLiteral` /
  `CommandPrefix` / `VarWrite` / `VarRead` positions through
  `Lowerer::command_surface`. A declared `script:body` word makes the call a
  `Statement::Barrier` that still carries its script; a declared `var` word
  becomes a `Statement::Call` def, which is what keeps `W210` off a variable
  the command writes.
- **The interprocedural scan** resolves the same roles through
  `ScanCtx::surface` and recurses into a call's `Body` and `Expr` words
  (`scan_role_code_arguments`), for a plain `Statement::Call` as well as a
  `Statement::Barrier`, so the procedures a declared script or expression
  calls are edges of the enclosing procedure. Code that runs in another frame
  or namespace (`FRAME_REACH_TRAITS`, `DEFINES_PROCEDURE`,
  `DECLARES_NAMESPACE`, or an absolutely-spelled name word) belongs to the
  body unit that owns it; walking it here would invent an edge to a
  same-named proc in the caller's namespace (issues #977 / #980).
  `DocumentCommandSurface::command_prefixes` carries the declared callback
  positions into the same scan, so a declared `command_prefix` word names an
  edge too
  — at `AppendedArity::Unknown`, since a declaration states a position and
  no count.
- **The call-site scan** (`unit_scope`) resolves a call's `CommandPrefix`,
  `Body`, `LambdaLiteral` and `VarWrite` positions through
  `CallSiteScanCtx::surface`, and `collect_scope_var_facts` reads the same
  surface for the variables a call writes. The interprocedural parameter seed
  folds a parameter only when every *caller* passes the same literal, so a
  declared callback registration or script body has to count as a caller
  there exactly as a catalogue one does — otherwise the seed sees a
  uniformity that the runtime does not have and `I230` fires on a live
  branch. `collect_call_site_constants` takes the surface from
  `UnitBuildOptions`; `scan_source_call_sites` takes the scanned file's own,
  since the declarations that bind a call site are the ones in the file the
  call site is written in.
- **The analyser** asks the same surface through `Analyser::command_surface`
  for its generic body walk and for its expression dispatch, so a declared
  body's commands resolve and a declared expression draws the expression
  diagnostics.

`tcl_compiler::analyser::utils::document_declared_surface` is the one
ingestion path all of them use: the analyser for its own
`declared_commands`, and every host that supplies a unit through the
`cu_override` seam (`tcl diag`, `tcl_lsp_db`, `xtask fp_sweep`), so the unit
it supplies declares exactly what the analyser's own unit would.

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
| `rust/tcl-compiler/src/analyser/utils.rs` | `scan_source_for_stubs`, `scan_sidecar_stubs`, `document_declared_surface` |
| `rust/tcl-compiler/src/compilation_unit.rs` | `UnitBuildOptions::declared_commands`, `CompilationUnit::declared_commands` |
| `rust/tcl-compiler/src/lowering/mod.rs` | `Lowerer::with_declared_commands`, `Lowerer::command_surface` |
| `rust/tcl-compiler/src/interprocedural.rs` | `ScanCtx::surface`, `scan_role_code_arguments` |
| `rust/tcl-compiler/src/unit_scope.rs` | `CallSiteScanCtx::surface`, `note_surface_var_writes` |
| `rust/tcl-compiler/src/analyser/state.rs` | `Analyser::command_surface` |
| `rust/tcl-compiler/src/analyser/types.rs` | `StubCommandDef`, `StubArgDef`, `StubExprDef`, `StubFlags`, `declared_traits`, `declared_side_effects` |
| `rust/tcl-compiler/src/side_effects.rs` | `classify_side_effects_in` |
| `rust/tcl-compiler/src/analyser/bounds_checks.rs` | `loop_shape` |
| `rust/tcl-lsp-core/src/minify.rs` | `find_rename_barriers` |
| `rust/tcl-registry/src/model/declaration.rs` | `DeclaredCommand`, `DeclaredArgument`, `DeclaredSurface`, `DocumentCommandSurface`, `role_for_word` |
| `rust/tcl-spec-studio/src/render_stub.rs` | stub rendering |
| `samples/` | example sidecar and inline stub files |
