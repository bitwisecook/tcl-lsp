# Current state

Audit basis: `work` at `127dba9d9`, 12 October 2026. This is a source inspection,
not a claim that all tests passed or the oracle matrix ran. Refresh the basis
when changing this document. GitHub issue status is recorded separately.

## What exists

- `tcl-core-types` provides command/namespace IDs, `CommandSlot` and the
  low-level variable scanner. `tcl-runtime-api::command_identity` provides
  static slot types, display projections and an injective encoding.
- `tcl-syntax::naming` owns written-name grammar, colon-run handling,
  variable/array source helpers and static command candidate order.
- `tcl-cmd-core::namespace` owns generic namespace operations over runtime
  adapters. The runtimes have token/tree storage and live lifetime handling.
- Compiler binding facts, registry resolved invocation facts, analyser
  indirection, LSP definition selection and workspace settlement provide
  common semantic layers for many consumers.
- Command/variable/namespace-op vector tables and C Tcl runners exist;
  command vectors also reach analyser, VM and WASM dispatch tests.
- Owner-resolution and resolution-drift gates exist, with limited coverage.

Static compiler/LSP adoption is incomplete: the static slot type exists, but
these crates still expose and compare flat `qualified_name` records. Do not
claim the records derive from an authoritative slot without inspecting the
producer and each use. Token-based runtime adoption does not fix static maps.

## Concrete residuals

Each entry is a verified code shape to resolve or justify. An absolute-name
predicate or display formatter may be legitimate; classify its inputs and
purpose before replacing it.

| File / function | Obligation |
| --- | --- |
| `tcl-compiler/src/var_resolve.rs::bind_scalar` | Leading-`::` qualification test, namespace normalisation and `rfind("::")`; relative-qualified `n::x` otherwise becomes local. Audit empty/lone-colon names and top-level plain/rooted aliasing. |
| `tcl-compiler/src/codegen/values.rs::is_qualified` | Leading-`::` test feeds stack/slot loads/stores, statements, control flow and substitutions. Distinguish qualified from absolute semantics at every caller. |
| `tcl-compiler/src/type_infer.rs::constructor_object_type` | Private candidates try global before current namespace and normalise a joined name. |
| `tcl-compiler/src/alias.rs::resolve_alias` | Private absolute/current/global selection without the path tier. |
| `tcl-compiler/src/auto_path_eval.rs::{canon, qualified_key}` | Private `split("::")` canonicalisation throughout load-time lookup. |
| `tcl-compiler/src/optimiser/helpers/naming.rs::namespace_from_qualified` | Reparses qualified strings; establish whether they are source, encoding or display. |
| `tcl-compiler/src/command_binding.rs`, `dynamic_names.rs` | Private import/forget/name-tail splits; preserve grammar and identity. |
| `tcl-lsp-core/src/signature_help.rs::resolve_alias_chain` | Private alias walk with its own eight-hop bound and FQN construction. |
| `tcl-lsp-core/src/{namespace_symbol, namespace_rename}.rs` | `split("::")` and two-byte separator assumptions reconstruct source spans incorrectly for colon runs. |
| `tcl-lsp-core/src/workspace_index.rs` | Repeated root trimming in identity/link keys and comparisons; preserve literal colon segments and interpreter/context identity. |
| `tcl-lsp-core/src/minify.rs::find_proc_call_sites` | Unresolved calls can fall back to literal name equality. |
| `tcl-lsp-core/src/type_definition.rs::find_class` | Manual rooted/unrooted adapter needs the common identity contract. |
| `tcl-mcp/src/tools.rs::generate_docstring` | Private FQN resolution and waived lexicographic simple-name fallback. |
| `tcl-lsp-server/src/lib.rs` | Private command-tail sets and stripped-name matching in workspace consumers. |
| `tcl-vm/src/interp.rs::resolve_command_fqn_uncached` | Token-aware current/path/global algorithm is independently implemented. Extract policy through a state adapter without losing retained/dead namespace rules. |
| `runtime/rust/src/namespace.rs::home_of` | Independent byte/tree candidate selection; preserve byte exactness and dying namespace edges. |
| VM `command.rs`, `cmd_math.rs`, `cmd_mathop.rs`, `cmd_oo.rs` | Tail/root splitting and qualification tests need representation-by-representation review. |
| Runtime `engine.rs`, `interp.rs`; WASM `backend.rs` | Root trimming and qualification checks need equivalent review. |
| `tcl-runtime-api::Namespaces` byte defaults | Lossy fallback conversions require an explicit supported representation and byte-exact overrides where supported. |

The complete search scope and consumer acceptance table are in [plan.md](plan.md).
These examples are the beginning of the per-call-site ledger, not an exhaustive
claim about every line in the repository.

## Oracle coverage gaps

The authoritative C pins are 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1.0 in
`rust/tcl-dialect/data/reference-toolchains.tsv`. Require all five for final
conformance evidence. Partial available-interpreter tests cannot prove the matrix.

The shared name runners have no latest-Jim discovery/pin leg. Existing Jim
syntax/surface measurements do not prove all name semantics. Variable and
namespace-op shared renderers do not yet have uniform consumer coverage in
both runtimes. Some runtime tests conditionally skip unavailable C binaries. Shared syntax
vector runners call `available_tclshs()`, which also skips missing/invalid
references and does not honour `TCL_LSP_REQUIRE_TCLSH`; only witness-based
callers enforce that variable. A full-matrix preflight/assertion is required.

BIG-IP has retained parser/context experiments under
`scripts/dev/bigip-probes/`; they are evidence for their exact appliance and
context. New name questions require explicit acceptance and execution records,
returned through [the F5 request](../../../experiments/name-handling/f5/REQUEST.md).

## Documentation contradictions to resolve

- `analysis/name-resolution.md` claims authoritative static slots and universal
  MCP provider adoption beyond the inspected implementation.
- `contracts/command-resolution.md` describes direct VM string-helper dispatch,
  clone-model imports and universally unbound codegen; verify current token
  and direct/native code paths before rewriting these statements.
- `compiler/namespace-resolution.md` mandates universal namespace normalisation
  for command keys and claims none of its static consumers model namespace path.
- `contracts/command-binding-and-aliasing.md` separates relative-qualified
  names from path lookup, contradicting the command resolution contract.
- C source citations use 8.6.16 and 9.1b0 while the current pins differ.
  Preserve old measurements as evidence; refresh current-source citations.
- `var_conformance.rs`, vector headers and runtime variable comments attribute
  fallback removal to TIP 278; the C reference document disputes that attribution.
- Runtime API docs say several implemented adapter APIs are not implemented;
  vector comments exclude lifecycle states now represented by runtimes.
- Long prior-bug narratives, milestone labels, banners and migration language
  remain in comments and documents.

Use source and real oracle observations to settle these disagreements. Do not
edit a test expectation merely to agree with the current implementation.
