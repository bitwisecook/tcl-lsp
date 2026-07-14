# Contract: command-name resolution — one algorithm, everywhere

Every component that answers "which command does this call dispatch?" must
implement **C Tcl's** rule, and must prove it by passing the shared
conformance vectors. This contract names the algorithm, its single Rust
home, every consumer, and the anti-drift gates.

## The algorithm (C Tcl 9, `Tcl_FindCommand`, `generic/tclNamesp.c`)

Given a call to `name` from current namespace `ns` whose
`namespace path` is `P1 … Pn`:

1. **Absolute** (`::`-led) name: resolve from the global namespace only.
   The path is never consulted.
2. **Relative** name — bare (`helper`) *or* qualifier-carrying
   (`inner::p`), identically: try, in order, `ns`, then `P1 … Pn`, then
   the global namespace as the base; the call dispatches the **first base
   under which the command exists**.
3. Existence means the *command* exists. A qualifier namespace merely
   existing does not commit resolution: `inner::p` from `::outer`
   dispatches `::inner::p` even when the namespace `::outer::inner`
   exists but holds no `p`.
4. No implicit ancestor walk: `helper` inside `::a::b` never reaches
   `::a::helper` unless `::a` is on the path.
5. Resolution happens at **call time**: a candidate defined later in the
   file (or dynamically) wins when the call finally runs.

Behaviour pinned against real tclsh 8.6.16 and 9.0.4 (they agree on every
case, including all edge cases above).

## Canonical implementation

[`tcl_syntax::naming`](../../../rust/tcl-syntax/src/naming.rs):

| Helper | Role |
|---|---|
| `command_resolution_candidates(ns, path, name)` | Candidate list in priority order (absolute → single candidate; relative → `ns`, path entries, global; deduped). |
| `bareword_resolution_candidates(ns, name)` | Path-free wrapper for consumers that do not model `namespace path`. |
| `resolve_command_with(ns, path, name, exists)` | The full rule: first candidate for which `exists` is true, `None` = `invalid command name`. |
| `naming::conformance` | The shared vector table (`tests/data/command_resolution_vectors.txt`), its parser, and `vector_script` (renders a vector as runnable Tcl). |

## Consumers and their conformance gates

| Consumer | How it conforms | Gate |
|---|---|---|
| Analyser call-site settlement (`Analyser::finalise_invocation_resolutions`, feeds `resolved_qualified_name` for references / rename / call hierarchy / code lens / symbol graph / minifier) | calls `resolve_command_with` post-walk (call-time semantics: whole-file definitions count) | `tcl-compiler/tests/command_resolution_conformance.rs` |
| Analyser shadow/arity checks (`resolve_proc_call`), W-code validity (`qualify_candidates`) | shared candidate helper | unit tests in `handlers.rs` / `validity.rs` |
| Optimiser interprocedural identity (`resolve_internal_call` / `resolve_call_target`), O103 folding (`resolve_proc_qname`) | `resolve_command_with` over the unit's proc table | unit tests in `interprocedural.rs` |
| `uplevel` passthrough inliner (`inline_uplevel.rs`) | `resolve_command_with` over the candidate map | existing inliner tests |
| Bytecode VM dispatch (`tcl-vm`: `lookup_command` = `resolve_command_fqn` + fetch) | `resolve_command_with` over the live command table, with the namespace's real `namespace path` | `tcl-vm/tests/command_resolution_conformance.rs` (compiles + executes every vector) |
| WASM runtime dispatch (`runtime/rust`: `Namespaces::home_of`) | structural mirror (its store is a namespace *tree*, not a flat map) — same base order, command-existence-checked per base | `cmd_namespace.rs::dispatch_matches_every_conformance_vector` (executes every vector) |
| eBPF backend (`bpf-tcl-*`) | **N/A** — no user procs, no namespaces; the 24-verb DSL rejects anything else as a hard compile error, so there is nothing to resolve |
| Compiler codegen | **no static binding** — proc calls emit runtime name dispatch (`invokeStk`), inheriting the VM's conformance |

The vector table itself is pinned to C Tcl by
`tcl-syntax/tests/command_resolution_conformance.rs::vectors_match_real_tclsh`,
which executes every vector under a real tclsh (`TCL_LSP_TCLSH`, else
`tclsh9.0` / `tclsh8.6` / `tclsh` on `PATH`; skips when none is
installed). **Adding a resolution behaviour = adding a vector**: every
implementation then has to pass it or fail its own suite — drift is a test
failure, not a code review hope.

## What changes which input, not the algorithm

These features interact with resolution by changing its *inputs* — they do
not change the rule, and must not grow bespoke resolution logic:

- **`namespace eval` / proc bodies / TclOO method bodies** select `ns`: a
  body resolves in its *defining* namespace (a method in its class's
  namespace), not its caller's.
- **`uplevel`** selects the *frame* whose namespace is current: the
  script resolves in the target frame's namespace (`#0` → global) —
  tclsh-pinned; both execution backends switch namespace with the frame
  (`tcl-vm::eval_at_level`, `runtime`'s `eval_uplevel`).
- **`interp alias` / `rename` / ensembles / imports** mutate the command
  *table* the existence check consults (an alias or renamed command is
  simply present under its name). See
  [command-alias-resolution.md](command-alias-resolution.md) and
  [command-binding-and-aliasing.md](command-binding-and-aliasing.md).
- **Execution traces** observe dispatch; they never alter it.
- **`upvar` / `variable` / `namespace upvar`** are the *variable*
  resolution surface, which is deliberately **not** this rule — a
  qualified variable write commits to the first namespace its qualifier
  resolves to, with no existence-checked fall-through (see
  `runtime/rust`'s `var_home` and
  [namespace-model.md](namespace-model.md)).

## Known modelling gaps (static side)

- The **analyser does not track `namespace path` declarations**, so its
  settlement conservatively passes an empty path (path-carrying vectors
  are skipped in its conformance test; both execution backends cover
  them). If the analyser gains path tracking, remove that filter and the
  gate tightens itself.
- **Custom command resolvers** (`Tcl_SetNamespaceResolvers`, C-level) and
  `namespace unknown` handlers are out of scope for static resolution;
  the runtime's `unknown` fallback fires only after this rule misses.
- Command names held in **variables** (`set cmd helper; $cmd …`) are
  statically undecidable and stay unresolved (pinned by LSP tests as a
  documented limitation).
