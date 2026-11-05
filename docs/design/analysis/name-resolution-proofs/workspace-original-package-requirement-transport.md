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

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim, bigip

Status: `not-tested`.

This is a Rust package-consumer integration contract. No C Tcl, Jim or BIG-IP execution of this question is claimed.

## Consumer bindings

- [rust/tcl-lsp-core/src/workspace_index.rs](../../../../rust/tcl-lsp-core/src/workspace_index.rs), `WorkspacePackageRequire::original_advice`: Retain original or Unknown key, every version alternative, exact selection and independent preference latch.
- [rust/tcl-lsp-core/src/workspace_index.rs](../../../../rust/tcl-lsp-core/src/workspace_index.rs), `original_package_requirement_advice_for`: Project the current document-owned complete package requirements.
- [rust/tcl-lsp-core/src/source_graph.rs](../../../../rust/tcl-lsp-core/src/source_graph.rs), `ancestor_requirements`: Traverse document ancestors without encoding requirement records as display strings.
- [rust/tcl-lsp-core/src/source_graph.rs](../../../../rust/tcl-lsp-core/src/source_graph.rs), `descendant_requirements`: Retain complete requirement records at independent source statement positions and body bounds.
- [rust/tcl-lsp-core/src/workspace_index.rs](../../../../rust/tcl-lsp-core/src/workspace_index.rs), `workspace_index::tests::original_requirement_graph_keeps_constraints_preference_and_unknowns` (linked): Checks exact opaque key, complete version alternatives/exact flag, local/caller preference, Unknown, cyclic ancestor transport and removal.

## Evidence

- [rust/tcl-lsp-core/src/workspace_index.rs](../../../../rust/tcl-lsp-core/src/workspace_index.rs)
- [rust/tcl-lsp-core/src/source_graph.rs](../../../../rust/tcl-lsp-core/src/source_graph.rs)
