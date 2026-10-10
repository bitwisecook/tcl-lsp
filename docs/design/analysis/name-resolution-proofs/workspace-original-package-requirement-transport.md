# naming.workspace.original-package-requirement-transport

Kind: `implementation-contract`

## Problem statement

A workspace require projected to a display name or bare package key loses exact-version matching, alternative version requirements and preference. A computed name must stay Unknown, and source graph cycles must preserve independently owned requirement records without manufacturing execution order.

## Question

Does the workspace requirement projection and generic ancestor graph retain original package identity, all version constraints, exact matching, local/caller preference and explicit Unknown?

## Conclusion

WorkspacePackageRequire retains an optional original package key plus requirements, exact and local preference separately from its report label. original_advice preserves Unknown and combines the local Latest latch with independently supplied entry preference. The generic ancestor/descendant graph transports complete caller-owned requirement records and positioned source bounds. Document removal withdraws its advice. Reachability and these projections supply source assistance; they do not prove completed require, package loading or a native interpreter preference.

## Scope

Current Core implementation contract. The exact regression distinguishes an opaque surrogate-unit package key, exact1.2 from alternatives1.0/2.0, Stable from local/caller Latest, computed Unknown, a source cycle and document removal. The named test asserts ancestor transport; positioned descendant transport is an inspected shared owner, not an asserted outcome of this regression. No Rust test outcome or native observation is claimed.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This is a Rust package-consumer integration contract. No C Tcl, Jim or BIG-IP execution of this question is claimed.

### tcl8.5

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This is a Rust package-consumer integration contract. No C Tcl, Jim or BIG-IP execution of this question is claimed.

### tcl8.6

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This is a Rust package-consumer integration contract. No C Tcl, Jim or BIG-IP execution of this question is claimed.

### tcl9.0

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This is a Rust package-consumer integration contract. No C Tcl, Jim or BIG-IP execution of this question is claimed.

### tcl9.1

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This is a Rust package-consumer integration contract. No C Tcl, Jim or BIG-IP execution of this question is claimed.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Jim Tcl.

This is a Rust package-consumer integration contract. No C Tcl, Jim or BIG-IP execution of this question is claimed.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: F5 iRules.

This is a Rust package-consumer integration contract. No C Tcl, Jim or BIG-IP execution of this question is claimed.

## Exact evidence

- `workspace-requirements` (implementation): [rust/tcl-lsp-core/src/workspace_index.rs](../../../../rust/tcl-lsp-core/src/workspace_index.rs). SHA-256 `806bbb1db56e6e3b1b92d411bd63a0ba18c642dace68c9d05b90d51edb424276`. Inspected original requirement metadata/projection and exact constraints/preference/unknown/withdrawal regression.
- `generic-source-graph` (implementation): [rust/tcl-lsp-core/src/source_graph.rs](../../../../rust/tcl-lsp-core/src/source_graph.rs). SHA-256 `7d6c7bd5d966018be3acc2235af12d18874c24a0ed78d415e51941aeb6cb9747`. Inspected generic ancestor/positioned descendant requirement transport retaining complete K records.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-lsp-core/src/workspace_index.rs](../../../../rust/tcl-lsp-core/src/workspace_index.rs), `WorkspacePackageRequire::original_advice`: Retain original or Unknown key, every version alternative, exact selection and independent preference latch.
- [rust/tcl-lsp-core/src/workspace_index.rs](../../../../rust/tcl-lsp-core/src/workspace_index.rs), `original_package_requirement_advice_for`: Project the current document-owned complete package requirements.
- [rust/tcl-lsp-core/src/source_graph.rs](../../../../rust/tcl-lsp-core/src/source_graph.rs), `ancestor_requirements`: Traverse document ancestors without encoding requirement records as display strings.
- [rust/tcl-lsp-core/src/source_graph.rs](../../../../rust/tcl-lsp-core/src/source_graph.rs), `descendant_requirements`: Retain complete requirement records at independent source statement positions and body bounds.
- [rust/tcl-lsp-core/src/workspace_index.rs](../../../../rust/tcl-lsp-core/src/workspace_index.rs), `workspace_index::tests::original_requirement_graph_keeps_constraints_preference_and_unknowns` (linked): Checks exact opaque key, complete version alternatives/exact flag, local/caller preference, Unknown, cyclic ancestor transport and removal.

A named test is a coverage binding, not a claim that it executed.

## Replay

The linked assertion must be executed under the coherent Rust workspace before claiming a passing result. Source hashes bind the inspected implementation and test; a source edit or formatter requires review/reissue. Package graph advice and provider selection do not establish that a Tcl loader or source file executed.
