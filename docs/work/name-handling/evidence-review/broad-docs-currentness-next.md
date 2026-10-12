# Bounded authored-document currentness cleanup

2026-10-12: edited only two durable Markdown documents and this temporary review.
No source, experiment input, oracle capture, proof record, index, Git mutation,
network call, build or full gate was performed. This is not a complete audit.

1. `docs/design/compiler/wasm-extensions.md`: replaced the hypothetical extension
   implementation plan with the current package-availability boundary. Removed
   status/temporal framing; retained the unsupported package-driven bundling limit,
   runtime ownership, Tcl-level tcltest and canonical runtime ABI distinction.
   Owners: `runtime/rust/Cargo.toml:67` declares `wasm_stdlib`;
   `runtime/rust/src/embedded_stdlib.rs:82` includes `tcltest/tcltest.tcl`;
   `runtime/rust/src/host_wasm.rs:106–129` supplies the VFS;
   `rust/tcl-compiler/src/codegen/wasm/pipeline.rs:54–74,544` declares packaging
   options and the shared pipeline, with no package-driven extension selection.

2. `docs/design/rust/salsa-interned-gc.md`: corrected obsolete acceptance claims.
   Current tests compare live slot counts after 24 and 96 edits, allowing at most
   2x growth; reuse counts are diagnostics, since healthy summary keys can report
   zero reuse on wider shard layouts. The raised control must exceed that bound
   for four body-keyed ingredients and report zero reuse for all six ingredients.
   Owners: `rust/tcl-lsp-db/tests/interned_gc.rs:112–158,369–446`.
   Added the current `EvaluatorEpoch` LOW-input rule and explained why one HIGH
   input need not suppress collection while another LOW read remains:
   `rust/tcl-lsp-db/src/lib.rs:163–175,438–443` and control at
   `rust/tcl-lsp-db/tests/interned_gc.rs:321–327`.
   Updated the inspected Salsa implementation reference from 0.28.2 to the
   locked 0.28.5 (`Cargo.lock:3538–3540`); local Salsa 0.28.5 `interned.rs`
   confirms LOW eligibility, query stamps, max retained durability and memo reuse.
   Removed completed assertion/implementation diary, preserving the scoped
   numerical reuse observations and issue links carrying measured provenance.

Validation: all 6 local Markdown file/anchor links resolve across the edited
files. No inbound links to their changed headings were found in docs/editors/ai
or AGENTS/CONTRIBUTING Markdown. `git diff --check` passed for both durable docs.
No tests were added or executed for these documentation-only corrections.

Deferred API-doc correction during source freeze:
`rust/tcl-lsp-db/src/lib.rs:183–191` still describes reuse-event acceptance and
only two HIGH control inputs. After freeze, replace that account with the current
24/96-edit live-slot comparison and three-input control described above.
Current why: the actual owner tests at `interned_gc.rs:369–446` and `:321–327`
make the crate comment stale; no source was edited or acceptance newly inferred.

Kept architecture W130–W134 reservation and interprocedural method-summary
limitations: inspected owners still retain them. No further durable docs were
selected. Repository-wide cleanup and removal of temporary work docs remain.
