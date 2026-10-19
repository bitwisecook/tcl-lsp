# Luna-26 review of Sol-21's Server102/103 and Server108 drafts

This was a static review only. No cargo was run, and the repository was not modified.

- **Patches apply:** `git apply --cached --check` under a temporary `GIT_INDEX_FILE`. Patch 1 applies on HEAD, and patch 2 applies on HEAD plus patch 1.
- **HEAD moved:** HEAD moved to `d6c3d5680` during the review. `git diff 19b8d7cc7 HEAD -- rust/` is empty, so the patches target the same code.
- **`nproc` = 4.**

## Patch 1: `server102-103-snapshot.patch` (ADMIT-WITH-CONDITIONS)

**Snapshot consistency holds.**
- `open_but_unindexed_symbols` (lib.rs:28489-28536) captures the index view and the open sources atomically under documents→index.
- Closed sources are revalidated against index revisions there.
- Rows and spans come only from captured data.
- `finish_workspace_symbol_response` only filters, so no live data reaches the response.

**What the patch changes for an "advanced" document.** An advanced document is one whose live version is greater than the captured version and whose revision did not go backwards. For these documents the patch skips:
- live equality;
- `uri_generation`;
- the index-revision check;
- the global DB revision.

**What is still checked for those documents:**
- `inputs_epoch`;
- factory generation;
- the configuration value (captured under the same `db` lock as `input`);
- the Salsa handle, the published receipt and the path;
- `document_analysis_input` `Arc::ptr_eq`;
- that the analysis matches its own input;
- `with_current` provider receipts;
- deleted or closed.

**The value checks cover what dropping the DB revision used to catch.**
- **Fresh-driver inputs:** all configuration fields are in `DiagnosticConfigurationSnapshot` (`tcl-lsp-db` lib.rs:748-777; server lib.rs:13695-13730, 10430).
- **Published factories:** guarded by `class_factory_generation` (lib.rs:6577-6595).
- **Per-file factories and subclass methods:** covered by input `ptr_eq`.
- **Folder configuration and packs:** these bump the epoch (lib.rs:25248, 27324).
- **Evaluator epoch:** a move is explicitly not an invalidation (lib.rs:5804-5811).

**Invalidations that are still refused:**
- configuration change;
- URI generation on a non-advanced document;
- close or rename (the old URI is gone);
- deleted file;
- dialect re-resolution without an edit (revision bump);
- dialect change on an advanced Database document (input `ptr_eq`).

**Sibling tests.** I read 98-107, 128-133 and the orphan test at lib.rs:60168. My predictions:
- Test 129 is refused by the configuration value.
- Test 130 is refused by the configuration value plus input `ptr_eq`.
- Test 131 is refused three ways: index revision, epoch, and raw equality.
- Code-action and coarse-token tests use a behaviour-identical refactor of `matches_current_state`.
- Reference, lens and hierarchy tests only gain a field that is never read.

**Gaps to state:**
1. **Close and reopen.** A close followed by a reopen at a higher version, when the captured revision was 0, counts as advanced, because the check is `>=` on revision. The 102/103 fixtures need this: they insert `with_version(...,2)` with revision 0, whereas a real `did_change` bumps the revision. The answer is still self-consistent and its receipt is checked, but NOTES' "dropped only if closed" is not exact.
2. **Closed documents.** Closed documents whose disk bytes or index revision changed are still dropped. This is defensible because they have no version order, but it is stricter than a literal reading of "drop only if invalidated". Tell the user.

**Conditions:**
- Run 102, 103 and the siblings listed above, plus `make rust-check`.
- Update `docs/design/contracts/name-handling.md`, section "Workspace-symbol request input currency". It still promises the "actual global DB revision" and "validates each captured actual document revision".
- Update `shared-utility-contracts-rust.md:637` ("exact configuration/DB revision").
- Document the reopen-at-revision-0 acceptance in `open_document_advanced`.

## Patch 2: `server108-cache.patch` (HOLD)

**1. It does not use the mechanism the user chose for Server108.**
- The fixture inserts Indexed open documents with no version and no `SourceFile` (lib.rs:65033-65075).
- The memo branch is never taken, and all 25 unchanged documents are re-analysed.
- The flip comes only from `buffered(min(nproc,16))` parallelism. That is not "re-analyse only changed documents".
- The flip depends on the hardware:
  - 1 core fails;
  - 4 cores here gives about 7 rounds × ~6.5 s (debug), plus the fixture;
  - under CI contention the result is uncertain.
- Nothing has been run.

**2. NOTES' "edits cancel them at item boundaries" is wrong here.**
- Live publication waits for an empty snapshot census before it enters a Salsa setter (lib.rs:1345-1355, 11434-11441, 11515-11526). So no cancellation fires.
- A memo read on a cold, closed hit file therefore delays every live edit publication until the analysis finishes.
- Base workspace/symbol held no Salsa snapshot.

**3. Cache eviction.**
- `file_analysis_incremental` is `lru = 64` (`tcl-lsp-db` lib.rs:4206).
- A broad Ctrl+T query that hits more than 64 closed files evicts the memos of open documents, which hover, semantic tokens and diagnostics depend on.
- Across keystrokes it thrashes with no reuse.
- This contradicts the warm contract that deep state is an open-document cost (lib.rs:29298-29315).

**4. No global bound.**
- `buffered(k)` bounds a single request only.
- Each cancelled or superseded request can leave up to k detached `spawn_blocking` analyses; base left 1.
- The warm keeps its bound global through `warm_task` abort.

**5. Parallelism itself is sound.**
- Hit order is preserved, and the first `Err` in hit order is returned. Unlike the serial base, all captures run before the error is reported.
- Lock order is unchanged, and I found no cycle.

**6. Producer drift.**
- The memo sees `external_call_sites`, which the fresh driver did not.
- Once DB-revision equality is removed, nothing re-checks it.
- Whether this can affect symbol rows is not argued.

**7. Sol-21's proposed fixture fix is not shown to fit the budget.**
- The proposal is `db_set_source` plus `analysis_for`.
- It moves about 25 cold analyses into the fixture, under the same 180 s budget.

**Next step: return the decision to the user, with three options.**
- (a) A fixture correction the user signs off, with measured timing.
- (b) An analysis cache owned by the index or the scan. This needs a contract decision.
- (c) Explicit approval of parallel capture, with a backend-wide semaphore.

If the memo reuse is kept, restrict it to open, warm documents.

## Q3: is the fixture's state reachable in production?

**For live open documents it is statically unreachable.**
- Open documents come only from `did_open`: `with_version`, `Pending`, with path admission (lib.rs:30282-30290).
- They reach Indexed only after Salsa publication.
- An open Indexed document with no input is a retired orphan, and orphans are skipped (lib.rs:28552-28560).

**The contracts and sibling fixtures still treat it as genuine.**
- The contracts call "no-handle" sources genuine: name-handling.md ~2072, shared-utility-contracts-rust.md:637, and readonly_reference_reports.rs:39 and 476.
- The sibling fixtures 103, 107 and 128-131 use the same shape.

A test-input change for 108 alone is therefore a contract-level decision. It needs the user's sign-off and cannot rest on lib.rs:955 alone.
