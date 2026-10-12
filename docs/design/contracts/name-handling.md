# Shared name handling

This is the entry point for consumers of Tcl names across the analyser,
compiler, LSP, middleware, runtimes, code generation, tooling and AI surfaces.
The implementation owners are listed in
[shared-utility-contracts-rust.md](shared-utility-contracts-rust.md).

## Representations

A name's spelling, identity and presentation are different values:

| Value | Meaning | Rule |
| --- | --- | --- |
| Written name | Source word or an evaluated Tcl value used for lookup | Parse with the selected dialect and the operation's grammar at ingress. Preserve the source span separately. |
| Namespace identity | A namespace in an interpreter, or a known static path | Retain its identity and lifetime. Its rendered name cannot identify a recreated namespace. |
| Command slot | Namespace identity plus the exact simple name | `CommandSlot` is the runtime representation; `StaticCommandSlot` expresses a statically known slot. A slot's occupant can change. |
| Command identity | A live command object, distinct from its table slot | Use `CommandId` and the runtime's generation/lifetime checks. Imports and traces can retain identity across renames. |
| Variable place/cell | Frame or namespace plus scalar/array identity and links | Resolve under the actual frame context; follow links through the variable owner. |
| Display name | Tcl-facing or user-facing rendered text | Render at output. Never parse it back into identity. |
| Compatibility key | Injective encoding for an adapter requiring string keys | Derive from authoritative identity; do not expose it as Tcl text. |

Distinct slots can render as the same FQN: namespace segment `a:` with
simple name `p`, and segment `a` with simple name `:p`, both render
`::a:::p`. No string normaliser can recover which slot produced that text.
The same applies to caches, workspace aggregation and editing identities.

Runtime stores retain structured slots. Compiler and LSP records still use
flat compatibility strings in several paths; their use requires a
representation audit. The existence of `StaticCommandSlot` does not establish
that a static consumer carries it.

## Choose the semantic owner

| Operation | Existing entry point / owner |
| --- | --- |
| Scan a source variable extent | `tcl_core_types::naming::scan_var_name_end_with`, selected by the dialect grammar |
| Parse colon runs and written command names | `tcl_syntax::naming::{qualifier_segments, canonical_written_command, written_command_tail}` |
| UTF-8 compatibility qualification | `naming::qualify`; only with its documented constructed-prefix precondition |
| Namespace-name canonicalisation | `naming::normalise_qualified_name`; unsuitable as a universal command/variable identity normaliser |
| Static command candidate order | `naming::{command_resolution_candidates, resolve_command_with}`; path-free wrapper only when the path is proven absent |
| Shared static slot projection | `tcl_runtime_api::command_identity::{StaticCommandSlot, display_namespace, display_command, encode_command_slot}` |
| Runtime namespace and command lookup | `tcl_runtime_api::Namespaces::{find_namespace, find_namespace_bytes, find_command, find_command_bytes}` through the runtime adapter |
| Namespace command semantics | `tcl_cmd_core::namespace`: which, origin, import patterns, variable FQNs, qualifiers/tail, parent/children/exists |
| Binding facts and safe specialisation | Compiler `command_binding`, registry `resolved_invocation`, dispatch proofs and site claims |
| Internal single-document target selection | `tcl_lsp_core::definition::{resolve_called_proc, resolve_proc_target_at, resolve_class_target_at}` (`pub(crate)` provider helpers; external crates use public providers/analysis facts) |
| Workspace settlement and import lifecycle | `workspace_index`, server `settle_call_against_workspace`, `namespace_import`, analyser `indirection` |
| Variable source syntax | Dialect-aware helpers in `naming`; array element splitting and word-substitution owners |
| Variable storage context | Compiler `var_resolve`/`var_scoping` and place/alias owners; runtime frame/cell owners |

Runtime lookup currently has separate structural implementations over token
arenas and byte-valued namespace trees. The shared string candidate helper
is not their direct implementation. Preserve storage adapters and identity
when consolidating policy; flattening a tree into display FQNs loses semantics.

## Context and observable semantics

Command lookup needs the current namespace, ordered namespace path, selected
release/dialect, live command existence and interpreter domain. Absolute
names search globally. Relative names, including `inner::p`, use
current/path/global candidate order, with command existence deciding the
winner. There is no implicit ancestor search. Tcl 8.4 lacks `namespace path`.
See [command-resolution.md](command-resolution.md).

Variable lookup needs the frame kind, selected release, lookup operation,
namespace, array syntax and alias links. `n::x` is qualified even without a
leading `::`; an unqualified variable in a proc is ordinarily frame-local.
Tcl 8.x namespace-scope fallback differs from Tcl 9.x. Reads, writes,
creation, introspection and declaration operations must not be conflated.
See [runtime-variable-frame-model.md](runtime-variable-frame-model.md) and
[compiled-scope-and-name-lowering.md](compiled-scope-and-name-lowering.md).

Renames, imports, aliases, ensembles, traces, unknown handlers and namespace
teardown modify bindings or lookup context. They use the same lookup policy,
with operation-specific identity and lifetime rules. `interp alias` retains
a target name and re-resolves it; an import retains command identity.
See [command-binding-and-aliasing.md](command-binding-and-aliasing.md).

Static consumers publish a resolved identity or a justified abstention with
its context. A display-tail match cannot turn an unresolved call into a
proved reference. Rename and refactoring require source spans and identity
proofs; source colon-run widths cannot be reconstructed from display names.
Per-command roles and semantics come from the registry, including overlays
and stubs, rather than spelling tests in consumers.

## Surface boundaries

LSP providers, server middleware, CLI tools and MCP tools consume common
analysis/lookup facts and adapt their presentation. Native/WASI/browser hosts
and editor clients must preserve those facts and their invalidation context.
AI prompt templates describe the same API contract; generated copies follow
their generator.

Compiler analyses and codegen consume binding and variable-place facts.
Direct/native specialisation needs a binding proof and guards; generic runtime
invocation preserves call-time lookup. Audit all registered backends before
claiming that every target delegates resolution. A restricted DSL such as
BPF-Tcl reports unsupported namespace/procedure features at its boundary.

F5 protocol command prefixes, iRule procedure namespaces and BIG-IP object
paths are distinct concepts. Preserve their registry/context rules. Appliance
`tclsh`, TMM iRules, tmsh CLI scripts and iApps are separate oracle contexts.
See [namespace-model.md](namespace-model.md).

## Evidence and enforcement

Shared command, variable and namespace-operation vectors live in
`rust/tcl-syntax/tests/data/`. Their C Tcl runners use `tcl-test-support` and
the release manifest in `tcl-dialect`. Consumer tests must derive expectations
from retained real-oracle observations for the supported release/context.
C Tcl, Jim and F5 can differ; expectations carry that axis explicitly.
The experiment protocol and executable observations are retained under
[experiments/name-handling](../../../experiments/name-handling/README.md).

`cargo xtask owner-resolution` checks owner/API/contract wiring.
`cargo xtask resolution-drift` checks a narrow class of namespace-blind
proc/class scans. Neither gate proves that every consumer has adopted the
owners; representation-aware consumer tests and a complete inventory are
also necessary.
