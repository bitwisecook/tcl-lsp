# Contract: retained positioned command realm

A lowered Module owns its original positioned source interpretation.
Consumers query that retained producer through
`realm::document_realm_bindings_for_module(module, registry)`.
The query returns a realm only when the original source image and channel,
complete grammar, entry and metadata input, namespace geometry and actual
Registry snapshot still correspond to the Module. It does not analyse source
again to repair a missing owner.

`RetainedSourceModuleBindings::positioned_for_module` supplies the checked
projection. Its immutable `Arc<SourceCommandBindings>` retains the original
Lowerer inventory. Derived inventory clones share its data; mutable branches
detach through the central copy-on-write owner. An earlier retained producer
cannot authenticate a changed Module.

## Propagate refusal

`None` from the checked Module query is terminal refusal.
`CommandBindingRealm::none/default` and optional helper contexts mean spelling
assistance; they cannot stand in for that refusal.

Caller analysis records reachable local callees as opaque when the projection
is unavailable. Interprocedural queries withdraw summaries and stop before a
taint memo runs. Whole-unit warning queries return no warnings. A refused
command-substitution callback stops before replay-name search or synthetic
compilation; other callback forms retain their independently selected path.
The raw optimiser withholds interprocedural results.

An in-place rewriter first retains the Module's existing owner `Arc`, then
checks its projection against the Module before moving statements. Its
candidate, implementation identity, frame, completion and derived-body guards
still apply independently.

## Actual new source ingress

Cross-file caller scanning lowers the actual new source once through explicit
source/configuration/entry ingress, then queries that newly produced Module.
Synthetic callback source also has its own genuine lowering ingress. Neither
route repairs a refused existing Module from printable text or partial options.

The [proof record](../analysis/name-resolution-proofs/retained-positioned-command-realm.md)
binds current owners and software definitions. The contract supplies source
correspondence only; Native lookup, evaluation, object freshness, entered
handlers, frames, stores, completion and edits require their own receipts.
No performance gain follows from these source definitions.
