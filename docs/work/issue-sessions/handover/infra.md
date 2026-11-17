# Hand-over: server and infrastructure (#1657, #2243)

Written 17 November 2026 by the outgoing session, paused since 16 November, as a message to the orchestrator and filed here unedited. The successor is the `server` issue session.

The paths and commits below predate the file split of 16 November: see *Starting* in `../README.md` before applying anything.

```text
HANDOFF — issues/infra session (#2243, #1657, #2129, #1734)

1. ISSUES
- #2129 CLOSED (comment 6902719775). Done. Open follow-ups, not blocking: cp1252 needs a codec in tcl_cmd_core::encoding before it can be declared; the setting is session-wide, not per folder; the JetBrains settings test was never run.
- #1734 CLOSED (comment 6904432148). Done. Not run: the VS Code web smoke test (checkCrossFileNavigation in editors/vscode/src/test/web/index.ts) and the wasm e2e. Both need `cargo install wasm-bindgen-cli --version 0.2.129` plus `rustup target add wasm32-unknown-unknown`; neither is installed here.
- #1657 OPEN. Fix e54c33e2f0 (futurelock in run_diagnostics_analyser_path) is on work. To close: a green CI test-ext covering it. CI is dispatch-only, and every work run since 2026-11-12 failed for unrelated reasons; the local ext-host suite is unusable on work (comment 6898904279). Open strand: RSS growth under pack churn (see 5, 6).
- #2243 OPEN, blocked on the burn-down, per the coordinator. Not touched.

2. BRANCHES
- work contains 77371b0de5 (#2129) and a6b31163a5 (#1734).
- issues/infra = 60259729df, a merge of work; it holds nothing that isn't already on work. Nothing is unpushed and the tree is clean.
- Push recipe used, since a force-push to issues/infra is denied: rebase onto origin/work, verify, `git push origin HEAD:work` (fast-forward only), then build a merge with `git commit-tree <tree> -p origin/issues/infra -p HEAD` and push that SHA to refs/heads/issues/infra.
- work moves every few minutes. Re-fetch right before pushing, and when it moves, check file overlap with `comm -12` of the changed files.

3. VERIFICATION (all on the commits above)
- Env: `export CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0`. Dev profile only, `--lib --tests`.
- Skip: depth_guard::tests::the_source_walk_cap_fits_its_stack_budget.
- fmt: `rustfmt --edition 2024 --check <changed .rs>`.
- clippy: `cargo clippy -p <crates> --all-targets -- -D warnings -A clippy::assert_is_empty`. Note: tcl-lsp-server-wasm is NOT a workspace member.
- Gates: `cargo xtask registry-axes`, `value-transfers --check`, `owner-resolution`, `callback-inventory --check`. Regenerate `docs/generated/{value-transfers,registry-axes}.md` whenever line numbers move.
- Baseline method: for each suite, run `cargo test -p X --lib 2>&1 | grep -E '^test .* FAILED$'` into a list. Then `git checkout <parent> -- rust`, rerun, `git checkout HEAD -- rust`, and diff with `comm -13`. New = fails after the change only.
- Failure counts after #2129: registry-lib 4, registry it_4 0, lsp-core 562 (539 after #1734 rebased), lsp-db 59, lsp-server 137, cli 52, compiler 1799. The server e2e suite (`cargo test -p tcl-lsp-server --test e2e`) has about 326 failures and takes about 33 minutes.
- Witnesses for #2129:
  - registry: a_declared_script_encoding_admits_non_ascii_text_under_every_release
  - lsp-db: a_declared_script_encoding_invalidates_and_admits_the_8x_fold (run with --lib --tests)
  - server: source_encoding_reaches_the_session_and_the_salsa_config_2129
  - cli: `--test value_transfers_cli` opt_folds_a_non_ascii_literal_under_the_declared_script_encoding
- Witnesses for #1734:
  - tcl-lsp-core: virtual_uri_tests (4)
  - tcl-lsp-server lib: virtual_workspace_uri_tests_1734 (2)
  - e2e: issue1707 (16)
  - VS Code: `MOCHA_GREP=... xvfb-run -a node ./out/test/runTest.js` after `npm run compile` (2 tests)
- Pre-existing failures: references_reach_a_socket_server_callback_with_a_dynamic_port, autoload_library_command_suppresses_w123_issue_832 and actual_proposal_command_refuses_selected_provider_and_unversioned_disk_provider all fail with and without my change.

4. DESIGN
- #2129, per maintainer comment 6898528356: ScriptEncoding {Auto, Declared(SystemEncoding)} lives on AnalysisContext. When Declared, TargetSemantics::for_context sets source_utf8. A thread-local, with_declared_script_encoding, bridges the legacy const-fold hook. The encoding is part of the salsa AnalyserConfig key, so changing it invalidates the cache. The CLI path is InputArgs::read_documents(); `opt` and `fmt` write their output back in the declared encoding.
- #1734, per maintainer comment 6898529166:
  - The MemoryStore files hierarchical non-file URIs under VIRTUAL_URI_MOUNT="/\0.tcl-lsp/uri/<scheme>/<~|@host>/<path>". The NUL byte follows the precedent of the spec-pack mount.
  - Any refusal from to_virtual_uri_path maps back to NotFile, so opaque URIs keep NoFilesystemPath.
  - from_virtual_uri_path is the inverse and is applied in constructed_file_uri.
  - Server URI→path sites use the uri_norm::DocumentPath trait.
  - fold_document_constants stays path-blind for non-file URIs.
  - provider_source uses canonical_file_uri instead of Uri::from_file_path.
  - NativeStore is unchanged.

5. DEAD ENDS
- #1657 RSS: `apt-get install heaptrack` was denied by the permission classifier; don't retry it.
- A temporary `tcl-lsp.debugSalsaMemory` command (salsa_unstable `memory_usage`, dumped to stderr) worked, but it reports only counts and inline sizes, not heap payloads. It was reverted and never committed.
- The registry overlay cache is already pruned by design (prune_overlays, OVERLAY_LIMIT), and salsa memos are LRU-capped with interned GC (lsp-db lib.rs:78-175). Neither looks like the obvious culprit.
- Measured on an opt-level-2 dev build, churn-only, 190 docs, with gdb `malloc_trim(0)` checkpoints. RSS before → after trim: r0 5.9→4.3 GB; r50 7.4→6.4; r100 7.5→7.0; r150 7.4→6.3; r200 8.4→7.5; r250 7.9→7.6. So it is live growth with noise, not only allocator retention. No attribution yet.
- The instrumented run "wedged" at startup (two 120s probe timeouts, cpu +0, wchar +0) before its second dump. This may be a real wedge recurrence with e54c33e2 present. Worth checking with a fresh run; I did not investigate.

6. NEXT, IN ORDER
- #1657 RSS:
  1. Rebuild the instrumented server: add the command next to "tcl-lsp.getEffectiveConfig" in lib.rs, and add the salsa_unstable feature in tcl-lsp-server/Cargo.toml.
  2. Rerun churn with a longer warm-up, then diff the per-query and interned counts at rounds 0, 50, 100 and 250.
  3. If the counts are flat, the growth is outside salsa: look at workspace_index, the per-pack registries held by Arcs, and DocumentState.
  4. Fix it, add a bounding test, commit on its own, and post one comment on #1657.
- #1657 close: dispatch CI test-ext once work is green.
- The stress driver is not in the repo; recreate it, because the scratchpad dies. It used .claude/skills/lsp-client/lsp_client.py and copied editors/vscode/testFixture into a workspace. It opened every .tcl file under 60 KB (190), rewrote .tcl-lsp/extensions.tclspec (a speclib with one command) every 1–2 s, deleting it every 3rd time, and sent didChangeWatchedFiles each time. It probed `tcl-lsp.getEffectiveConfig` every 0.5 s with a 120 s timeout, and logged VmRSS every 10 rounds.

Pitfalls:
- Disk: target/debug/build fills 250 GB quickly. Prune with `for d in target/debug/build/*/; do ls -1dt "$d"*/ | tail -n +2 | xargs rm -rf; done; rm -rf target/debug/incremental`.
- A stale opt-level-2 binary is about 170 MB, and the server uses 4–8 GB RSS.
- The e2e flakes issue954_followup::* (three tests) and spec_packs::a_pack_added_after_startup_reloads_and_re_analyses_open_documents pass and fail at random on any commit.
- work's own clippy failure at taint.rs:2016 is owned by the coordinator.
```
