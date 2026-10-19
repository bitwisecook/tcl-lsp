# Sol-33: parked analysis-reuse work (2026-10-19, user decision 12:30Z)

Patches: `patches/` (static; never compiled or run). Base: HEAD efdb1af7b (no rust/ or docs/design change since c2b28d6d8).
Each applies with `git apply --cached` on a temp GIT_INDEX_FILE, alone and in either order with the others. rustfmt clean, no `#[allow]`.

- `b1-v2.patch`: prefix fold in `global_instance_classes_with` (production change unchanged from v1, which Luna-37 judged correct). Test rewritten: called `::callable` against uncalled `::idle`, reusing `selected_global_option_taint_tests::build_tk_module`, which is now `pub(super)`.
- `c2.patch`: per-call `LexicalAnalyses` minify memo. Luna-37: ADMIT-TO-BUILD.
- `a1-a3-v2.patch`: `ReadonlyAnalysisMemo` (open tier plus bounded closed LRU). Luna-37's items are addressed in the draft, but nothing has been verified:
  - **R1, stub names.** `SidecarStubFingerprint` now probes exact names per ancestor directory. Names come from `sidecar_stub_names`: the dialect, the environment's analyser and unit profile names, and the supplied input's two profile names. Each probe records `(is_file, read_to_string().ok())`, mirroring `has_sidecar_stubs` and `scan_sidecar_stubs`. Directories are no longer listed. Before insertion the run's own `resolved_input` profile names must be a subset of the probed names.
    - Verify that `tcl_registry::model::ingress::resolve_environment` is the analyser's resolution (`environment_ingress` re-exports it).
    - Verify on macOS/Windows that the case-insensitive lookup behaves as the analyser's does.
  - **R2, stale generations.** Every entry shares one (epoch, factory generation). An insert with a newer pair clears the memo; an insert with an older pair is refused. `invalidate_diag_inputs` calls `memo.clear(new_epoch)`, which raises an epoch floor so runs that finish after the bump are refused.
    - Unchecked: whether the factory generation is monotonic at every publisher. The code assumes it.
  - **S1.** The memo keeps its own open set: `did_open` calls `promote`, `did_close` calls `demote`, and the tier is chosen on insert.
  - **S3.** Contract notes added to `docs/design/rust/lsp-performance.md` (closed-tier exception) and to `name-handling.md` (epoch covers global registry state).
  - **S4.** Entry points added to row 637, plus two source-manifest rows (`ReadonlyAnalysisMemo` → `PerUriCaches` `type`; `Backend::readonly_analysis_seed` → `capture_readonly_reference_document` `self`). Unverified: run `cargo xtask owner-resolution`.
- `a2-v2.patch`: shared `readonly_declaration_documents` for logical calls and lenses, the hierarchy walk and the reference report. Commit message: `patches/a2-v2.commit-message.txt` (records the Err→Ok(None) nuance).

Measurement items before merge:
- **Byte factor.** `ESTIMATED_ANALYSIS_BYTES_PER_SOURCE_BYTE = 64` plus 16 KiB per entry is unmeasured. Measure the retained heap of an AnalysisResult per source byte on the corpus, then fix the factor or keep the contract's "estimate" wording.
- **Closed-tier churn.** The closed LRU (default 256 entries) gives no reuse for workspace-symbol requests that touch more closed files than the bound.
- **Counter assertions** in the memo tests (runs 2, hits 2, +2 after an epoch bump; closed-tier-0 runs 3) and `concurrent_snapshots_share_one_text_and_index_allocation` (strong count 34) must be confirmed in a build.
- **Resume gates:** `make rust-check` (clippy pedantic, owner-resolution, name-resolution-proofs), then the tcl-compiler, tcl-lsp-core and tcl-lsp-server tests.

Superseded: `b1.patch`, `a1-a3.patch`, `a2.patch` (v1). Design: `DESIGN.md`. Work trees: `work/`.
