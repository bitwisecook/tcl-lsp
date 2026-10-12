# Does actual Registry store currency preserve original diagnostic invalidation?

Question: when an indexed document keeps its exact source, scanner configuration
and profile but its genuine immutable command store changes, does diagnostic
scheduling retain original provider/caller dependencies and conservatively keep
unknown relevant Tcl callers, while excluding JavaScript and BIG-IP configuration?

Canonical contract: `naming.editor.original-diagnostic-invalidation`,
[original-diagnostic-invalidation.md](../../../../../docs/design/analysis/name-resolution-proofs/original-diagnostic-invalidation.md).
All seven provider statuses are `not-tested`; both software controls are `linked`.
The current complete checker reports this stale Core binding at
[check.log:106](../20261012-current-consumer-check/check.log).

Only two actual consumer owners are in scope:

| Owner | Canonical bound SHA256 | Inspected current SHA256 |
| --- | --- | --- |
| `rust/tcl-lsp-core/src/workspace_index.rs`, evidence `implementation-0` | `feefb4b2da2ae821f3ebec02282bd7a92da59461cfbccfd7f09bf21aea6c13dc` | `2eb79c0c683e5bf9b0e5ca07d4f8d82ec6ef10eff2cb1b9f15140ac4a05cbed0` |
| `rust/tcl-lsp-server/src/lib.rs`, evidence `implementation-1` | `b397e577d15a2b3b3a6d2118843f0c77c0bae15bea34c73b2b5fdb3d15eb303d` | same |

Exact Core bound bytes are Git blob `ee44b30b2c770928452058838310aff8c61fc3d7`
at revision `6485ef15fc6ea8c86d50cf1d206977aa9591f393`. Current Core replaces
dialect-label Registry recapture with the document's guarded actual store and
adds its semantic key to warmed settlement dependencies. The source/context
and named original controls are distinct from acceptance of these changed paths.

Actual ports: [WorkspaceDiagnosticSourceContext:1182](../../../../../rust/tcl-lsp-core/src/workspace_index.rs),
[document Registry borrow:1378](../../../../../rust/tcl-lsp-core/src/workspace_index.rs),
[settlement dependencies:1390](../../../../../rust/tcl-lsp-core/src/workspace_index.rs),
[replace_document:3602](../../../../../rust/tcl-lsp-core/src/workspace_index.rs),
[IndexedDiagnosticFacts:6708](../../../../../rust/tcl-lsp-server/src/lib.rs),
[change_to:6823](../../../../../rust/tcl-lsp-server/src/lib.rs), and
[original_diagnostic_consumers:7030](../../../../../rust/tcl-lsp-server/src/lib.rs).

Required finite acceptance:

- Run the two canonical controls below on the current owned images. Preserve
  genuine D800/D801 caller separation, cleared reporting maps, source/configuration
  withdrawal, missing lookup conservatism, removal/reused slots and foreign exclusion.
- Run the two current Core store controls: warm a genuine Logical import link;
  replace only its same-profile actual store with a `Target::p` conflict, require
  changed settlement dependencies and link withdrawal, then restore the authentic
  owner. Foreign borrowed input and removed/reused slots must decline.
- Add one fixed Server control through actual `capture`, `change_to` and
  `original_diagnostic_consumers`: unchanged C8.6 source/config/profile, two real
  stores differing by an authored row, and authentic open Tcl/JS/config documents.
  Require a changed retained source context and `original_changed`. Isolate the
  no-user-declaration case using a real empty/comment source; assert the actual
  changed-slot set is empty, then require all relevant Tcl participants and no
  foreign buffers. A failed premise keeps this question open; never clear/donate
  original facts. Restore the genuine input; missing/stale callers retain Tcl.

After the existing Core/Server cache leases release, use their configured targets:

```sh
cargo test -p tcl-lsp-core --lib workspace_index::tests::diagnostic_source_context_retains_currency_and_clears_reused_slots -- --exact
cargo test -p tcl-lsp-server --lib tests::original_diagnostic_invalidation_keeps_opaque_dependencies_and_foreign_surfaces_separate -- --exact
cargo test -p tcl-lsp-core --lib workspace_index::retained_store_consumer_tests::actual_import_metadata_store_change_invalidates_warmed_same_profile_settlement -- --exact
cargo test -p tcl-lsp-core --lib workspace_index::retained_store_consumer_tests::indexed_registry_borrow_rejects_foreign_input_and_cleared_slot -- --exact
```

Retain exact argv, status/streams, binary hash and selected source/configuration
hashes before/after; attach the new control only after its actual definition exists.
Acceptance schedules diagnostics only: no Native binding/lookup, installed command,
Normal completion, package load or edit authority follows. No test outcome, source
renewal, provider promotion or acceptance for the other four Workspace questions
is supplied by this proposal. Canonical hashes and statuses remain unchanged.
