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
| `-extension` | a native extension registers the command | the conservative default of `tcl_registry::extension_default`: `EVALUATES_CODE`, `CREATES_BARRIER`, `CREATES_DYNAMIC_BARRIER`, `ESTABLISHES_VARIABLE_TRACE`, `TAINT_SINK`, `TAINT_SOURCE`, `UNSAFE` and `SAFE_INTERP_HIDDEN`, and an unknown read and write | everything that reads the flags above: the call is a dynamic barrier (`classify_side_effects_in`, the minifier's rename barriers), is hidden in a safe interpreter (`W129`) and is never pure; the other flags narrow it, as below |

A stub with no flags states no behaviour, and side-effect classification
treats it exactly as an undeclared command: an unknown read and write, never
pure.

### Frame effect

`-frame WORD` states what the command does to the frame that calls it, in the
registry's own frame-effect vocabulary: the value a catalogue command carries
on `CommandSpec::frame_effect` (a `tcl_registry::frame_effect::FrameEffectSpec`,
`None` for a command that crosses no frame).
`DeclaredFrameEffect::from_stub_flags` reads the flag run and
`DeclaredFrameEffect::from_stub_word` the word; `StubCommandDef::frame` holds
the effect, and it lands on `DeclaredCommand::frame_effect`, beside the traits
and side effects the flags state:

| Word | Frame effect | Meaning |
|---|---|---|
| `own` | `None` | the command's code runs in a frame of its own, as a procedure's does |
| `none` | `None` | the command runs no code that reaches a frame |
| `caller` | `FrameArgLayout::OpaqueCallerVars` (`DeclaredFrameEffect::CALLER`) | the command reaches variables of the frame that calls it under names nothing states, as `argparse` does |

The last `-frame` written wins. A word the grammar does not know states
nothing, as an unrecognised flag does; `-frame` sets no `StubFlags` bit.

`own` and `none` state what a catalogue command with no frame effect and no
variable argument states: the call sets, unsets and reads no variable the
frame that calls it can name, a global or a namespace variable included. A
command that reads its caller's variables, or touches a global or namespace
variable by name (`global`, `upvar #0`, a `::`-qualified name), states
neither: it leaves `-frame` unstated, or states `caller` when all it does is
set variables of the frame that calls it, as `argparse` does.

**Unstated is the default, and it widens.** A stub without `-frame`
(`DeclaredFrameEffect::Unstated`) says nothing of the frame that calls the
command, so nothing bounds it: like code the module cannot see, the command
may reach that frame through `upvar 1` or `uplevel 1`. A call to it is a call
to code the module cannot see: in a procedure every local the
procedure holds at the call takes a fresh, unknown version, and at the top
level every global does, so a constant held across the call is not folded
(`I230`, `O112`), a store the call may read is kept (`W211`, `W220`, `O109`),
and `[info exists]` after it decides nothing. That is what leaving the frame
effect unstated costs, and the conservative default it buys: a stub never
claims a frame effect its author did not state. `W123` on a head in a
procedure's frame ends with a sentence that says so and names the declaration
that keeps the locals.

**A plain call is a named head.** A declaration names its command to the flow
graph when it states its frame effect and states nothing else the flow graph
would have to read from it (`DeclaredCommand::plain_call_frame_effect`): every
argument is data — `value`, `name`, `pattern` or `channel` — and no flag but
`-pure` or `-unsafe` is written. The flow graph reads roles and traits off the
catalogue, which answers a name it does not hold as a command of plain values
that does nothing it models, and that answer is the declaration's own only for
such a stub. The lowering puts these commands on `Module::declared_frame_effects`
(`DocumentCommandSurface::plain_call_frame_effects`), by normalised qualified
name, leaving out a name the catalogue holds, which keeps the catalogue's
answer. The module's command table (`ModuleCommandBindings`) binds them beside
the registry's names, so `CfgBuilder::head_is_unseen` answers for a call to one
by its one rule, as for a catalogue command reached by its own spelling, and
the call brings its stated effect as a catalogue command's call brings
`CommandSpec::frame_effect`: under `own` or `none` nothing widens, and under
`caller` the computed-name walk (`dynamic_names`) reads
`CfgFunction::declared_frame_effects` and raises the barrier it raises for
`argparse`, for the function and from the statement on.

A declaration with a code or variable word (`body`, `expr`, `command_prefix`,
`var`, `var_read`) or another flag stays a call to code the module cannot see,
whatever its `-frame`: the lowering reads its roles, but the flow graph reads
roles and traits off the catalogue alone, so it would miss the body a
substitution such as `set n [db_eval $sql {set g 6}]` runs, or the variable a
computed name writes. A `body` or `command_prefix` word still answers as the
catalogue's rule answers for one written directly: a script whose timing is
not known makes the call the barrier `time {…}` lowers to (`unsupported body
command`), which widens every value, whatever the frame effect.

### Extension commands

`-extension` is for a command C code registers (`Tcl_CreateObjCommand`), of
which nothing can be known from where the analyser stands. The declaration
starts at the top of every axis the registry has a fact for — it may run any
argument as a script or name a variable at any level, read and write any
state, establish a variable trace, and complete with any code; it is a taint
sink and source, unsafe and hidden in a safe interpreter, and never pure — and
the facts the other flags state narrow it, each on its own axis and on no
other:

- A statement of *effects* replaces the effect axes (code evaluation, traces
  and the unknown read and write) by what it states. `-pure` states none, and
  `-mutator` states the variable read and write of its target, because a
  command that is pure, or that only rewrites its target, evaluates no code.
- `-barrier` beside either states the evaluation axis back, so a stated
  barrier survives a stated effect.
- `-loop`, `-scope_alias` and `-unsafe` state facts of their own and narrow
  nothing.
- No flag states the taint axis or the safety axis away: a pure extension
  command is still a taint source and still hidden in a safe interpreter.

A stub without `-extension` starts from nothing, as it always has. The
registry's own default for the same command (`CommandSpec::extension_default`)
declares no transition descriptor on purpose: a command with none is a wildcard
over every state domain, which a closed statement could only narrow.

Four consumers still read these facts off the catalogue alone, so a stub's
flags do not reach them yet: the optimiser's own elimination gate and GVN (a
`-pure` call's unused result goes only through the interprocedural summary,
so `set a [mypure $x]` written directly is kept), SSA's barrier-def walk
(which reads no declared role either), memory SSA's clobber verdict
(which treats every command the catalogue lacks as clobbering, flags or
not — the conservative answer), and the taint analysis, so an `-extension` stub
is not yet a taint source or sink to it.

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
`StubFlags` bitflag set, the frame effect the `-frame` word states, and the
`from_sidecar` marker. They are collected onto
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
| `rust/tcl-compiler/src/command_binding.rs` | the module's command table, which binds a declared plain call beside the registry's names |
| `rust/tcl-compiler/src/dynamic_names.rs` | the computed-name walk, which reads a declared plain call's frame effect |
| `rust/tcl-compiler/src/side_effects.rs` | `classify_side_effects_in` |
| `rust/tcl-compiler/src/analyser/bounds_checks.rs` | `loop_shape` |
| `rust/tcl-lsp-core/src/minify.rs` | `find_rename_barriers` |
| `rust/tcl-registry/src/model/declaration.rs` | `DeclaredCommand`, `DeclaredArgument`, `DeclaredFrameEffect`, `DeclaredSurface`, `DocumentCommandSurface`, `role_for_word` |
| `rust/tcl-spec-studio/src/render_stub.rs` | stub rendering |
| `samples/` | example sidecar and inline stub files |
