# Var-escape analysis

Var-escape analysis answers one target-independent question: can a Tcl
procedure variable be observed by name across a frame boundary? The answer is
used by optimiser and code-generation proofs, but the analysis is not a
code-generation entry point. [`compile_wasm`](wasm-codegen.md) is the sole
public Tcl-to-WebAssembly pipeline.

## Lattice and soundness rule

```text
Local  <=  Frame
```

- `Local` means every known access is statically resolved and no external
  observer needs the variable's Tcl name.
- `Frame` means the runtime frame must preserve name-based observation by an
  interpreter, alias, trace, or frame-crossing command.

`Frame` dominates at every join. Missing, dynamic, malformed, and unmodelled
evidence therefore abstains from optimisation rather than inventing a
`Local` proof.

The result is a `ProcEscapeSummary`. It retains:

- per-name `EscapeTag` values and, on the flow-sensitive path, per-SSA-version
  tags;
- typed `Barrier` and `EscapeReason` values with source spans where available;
- caller-frame names reached by `upvar`, direct callees, and the
  interprocedural fixed point;
- resolved indexed local slots where that representation is safe; and
- conservative predicates for inlining, dead-store elimination, and frame
  elision.

## Rust module layout

The implementation is under `rust/tcl-compiler/src/var_escape/`:

- `api.rs` orchestrates the analysis entry points;
- `cfg_propagation/` propagates flow-sensitive CFG and SSA facts;
- `walker.rs`, `handlers.rs`, and `state.rs` implement the IR tree walk;
- `interprocedural.rs` solves callee-induced escapes to a fixed point;
- `slot_resolution.rs` assigns safe indexed local slots;
- `info_subcommands.rs` centralises `info` frame-observation categories; and
- `types.rs` owns the typed public vocabulary.

Command semantics come from `CommandRegistry`. The production IR entry uses
`analyse_var_escape_with_registry`, so argument roles, barriers, and Tcl
profile differences remain registry-owned. Generic walkers do not recognise
commands by spelling.

## Entry points

```rust,ignore
analyse_var_escape(&module, interprocedural)
analyse_var_escape_with_registry(&module, interprocedural, registry)
analyse_var_escape_cu(&compilation_unit, interprocedural)
```

The first two walk lowered IR. The registry-aware form is the production path
used by inlining and computes the `pure_leaf` fixed point. The
`CompilationUnit` form consumes the existing control-flow graph and static
single-assignment graph, retaining versioned tags for consumers that need
flow-sensitive evidence.

These APIs return facts. They do not emit WebAssembly and are not alternate
backends. A code-generation optimisation must be selected inside
`compile_wasm` from the complete `CompilationUnit`; callers cannot inject a
parallel escape-summary side channel.

## Transfer principles

Registry descriptors and generic argument roles drive the detailed transfer
functions. The following principles define their conservative envelope:

| Surface | Proof consequence |
|---|---|
| Literal local read or write | Does not escape by itself. |
| Literal `upvar` or `global` relationship | Escapes only the names proven to participate. |
| Dynamic `upvar` level or source | Records an `Upvar` barrier and widens. |
| Literal evaluable body | Recursively analyses the body when parsing and lowering succeed. |
| Dynamic or unparseable `eval`, `source`, or `subst` body | Records an `Eval` barrier and widens. |
| Literal variable-name role | Escapes that resolved name when the command observes it by name. |
| Unbounded dynamic variable name | Records a `DynName` barrier and escapes all known locals. |
| Frame-inspecting `info` form | Records an `Info` barrier and widens. |
| Opaque expanded invocation | Records an `Expand` barrier when argument positions cannot be bounded. |

The `info` categories are centralised in `info_subcommands.rs`. Frame
inspection includes `level`, `frame`, `vars`, `locals`, `coroutine`, and
`errorstack`. Other Tcl 9 forms are classified separately because reading
interpreter-global metadata does not, by itself, reveal procedure locals.

## Flow-sensitive and interprocedural facts

The CFG path keys variable versions by `(name, ssa_version)`. At a physical
storage boundary it collapses all versions by joining them, so one escaping
definition makes the name `Frame`. The fine-grained `ssa_tags` remain
available to future consumers.

The interprocedural solver propagates literal caller-frame names reached by a
callee's `upvar`. An unbounded callee source widens the caller. It also computes
the transitive `pure_leaf` predicate: an opaque or impure callee can only
downgrade the result.

## Consumer contract

Consumers use the summary predicates instead of recomposing partial rules:

- `safe_to_inline` requires the transitive pure-leaf proof;
- `safe_to_dce` requires no external observer of procedure locals; and
- `safe_for_frame_elision` requires that the procedure frame is unobserved.

The current inliner consumes the registry-aware IR result. The CFG/SSA result
is a common analysis surface for code-generation and diagnostic work. The
canonical WASM pipeline must retain any decline as typed plan evidence and use
its private compatibility plan when a proof is unavailable. It must not expose
a second emitter or a user-selectable “legacy” path.

## Tricky Tcl surfaces

`upvar`, `uplevel`, dynamic `eval`, traces, command renaming, aliases,
namespaces, TclOO, safe or child interpreters, `unknown`, and runtime package
loading can all change observation or dispatch. Escape analysis covers only
its declared variable-observation proof. Dispatch stability, world state,
completion behaviour, side effects, and dual-ported Tcl object representation
remain separate common proofs. No single analysis may infer those properties
from an escape tag.

## Tests

Focused coverage lives in:

- `rust/tcl-compiler/tests/var_escape_cfg.rs`;
- `rust/tcl-compiler/tests/var_escape_residual2.rs`;
- `rust/tcl-compiler/tests/var_escape_typeinfer.rs`; and
- `rust/tcl-compiler/tests/inlining.rs`.

Tests cover literal and dynamic aliases, `info` forms, nested control flow,
SSA joins, recursion, interprocedural `upvar`, typed reasons, registry-aware
roles, and conservative fallback.
