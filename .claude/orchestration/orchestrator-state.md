# Orchestrator state — tcl-lsp full-remainder sweep

## Session facts
- Repo: bitwisecook/tcl-lsp at /home/user/tcl-lsp; HEAD = 5a035d208 = origin/rust (clean).
- My designated branch: claude/tcl-lsp-full-remainder-i1o1bx (currently == origin/rust).
- Workspace dir is still `rust/` (crates/ rename NOT landed → final-wave PR).
- Model policy: opus = implementation + engine/codegen verification/adversary; sonnet = mechanical/research/docs; fable = escalation only (B1 DSL design, #1372 shim arch, disputes).

## Toolchain (R0 reference card)
- tclvm: /home/user/tcl-lsp/target/debug/tclvm -c '<script>' [--tcl-version 8.4|8.5|8.6|9.0|9.1]
- runtime-rust: /home/user/tcl-lsp/runtime/rust/target/debug/examples/run_script <file.tcl> (file arg only; rebuilt with TCL_TOMMATH_DIR=/home/user/tcl-lsp/tmp/tcl9.0.4/libtommath, build-time only)
- tclsh 8.6.16 EXACT oracle: /home/user/tcl-lsp/tmp/tcl8616-install/bin/tclsh8.6 (system tclsh8.6 is 8.6.14 — do not use for parity)
- tclsh 9.0.4: tclsh9.0 (system, exact)
- tcl CLI (compiler only, no eval): /home/user/tcl-lsp/target/debug/tcl
- runtime/rust is EXCLUDED from workspace (own target dir, needs unsafe). Never share CARGO_TARGET_DIR across worktrees.

## R3 done (2026-08-18/19)
- CLOSED with evidence: #1407, #1416 (PR #1551).
- Scope comments posted: #1405 #1463 #1440 #1442 #1412 #1542 #1404 #1446 #1450 #1452 #1464 #1382 #1428 #1432 #1429 #1458 #1457 #1438.
- NEW issue filed: #1568 (VM compiled-word ${...} inversion) → E4 lane roster.
- #1473 tracker comment: PENDING. #1524: accurate per V-B, no update needed.
- PR-D re-scoped: #1405 residual is type-debt (tcl_expr_eval.rs + 381-site LSP-layer tail + closed-vocab enums); no collision with engine lanes → runs PARALLEL, not gating.
- #1399 decision: FIX bench.py (outer deadline + phase attribution), not retire.

## Implementation wave plan (disk-capped: ~19G avail → max 3 worktree lanes concurrent)
- Tranche 1 (LAUNCHED 2026-08-19): E4 = aa9e96d0a5a7bf404 branch claude/e4-codecs-subst (#1427 #1429 #1439 #1443 #1457 #1458 #1568); E2 = a87ccdc6ffef6554f branch claude/e2-namespaces (#1442r #1446+regr #1451 #1453 #1463-TclOO; must NOT touch Namespaces::rename seam — E5's); E3 = a8f198553f1c371f0 branch claude/e3-traces (#1438 #1440r #1444); B1-design = a31fe627a936c9be5 (proposal → scratchpad/b1-proposal.md → my review).
- B1 DESIGN DONE + REVIEWED (fable): APPROVED w/ 4 amendments (see b1-proposal.md header): wire-compat finding enum; graph.rs → B6; segmenter-divergence B1-decides; fork-6 driver → B5. B1-impl = sequencing items 1-4 (fresh worktree agent when slot frees — critical path for Track B).
- Tranche 2 (revised for critical path): E1 (#1382+ #1425 #1428+ #1432), E5 (#1412r #1450 #1452+ #1464+, + VM interp-hide adjacent), B1-impl (#1530 #1543 per approved proposal).
- Tranche 3: W (#1376 #1459 #1542-rescoped), PR-D (#1405 residual), B6 (#1527 #1528 #1529 residuals + graph.rs per fork-5 ruling), G (#1399 fix + #1404 items 1-3); then B2-B5 (B5 gets fork-6 driver), B7; then S (#1372, design first); final: perf check + rust/→crates/ layout PR.
- DISK PROTOCOL (2026-08-19): main checkout target dirs DELETED (freed 7.6G; R0 binaries gone — rebuild via R0 recipe if ever needed). Deep gates SERIALIZED across lanes: <6GB free → lane waits for my go-ahead. Logs redirected not tee'd; >500MB log = runaway, kill+investigate. E2 hit ENOSPC during prep-pr (12GB log — suspicious, told to investigate); E2 impl done+pushed (claude/e2-namespaces, 2 commits on 5a035d208), unit gates green, deep gates re-running now with the slot.
- GATE SLOT QUEUE: E2 (running) → E4 (holding, requested) → E3. E4 also hit ENOSPC pre-advisory, did sanctioned full-clean recovery (nothing pre-ENOSPC carried); 6/7 issues committed (#1443 #1458 #1427 #1429 #1439 #1457 + contracts). #1568 REASSIGNED E4→W lane (compiler territory): root cause = segmenter.rs:234 word_piece bail + values.rs:458 (9.x rule) vs helpers.rs:260 (8.x rule); fix = BracedVarStyle through CodegenCtx, ~41 sites/7 files; handoff spec → scratchpad/verdicts/1568-handoff.md (E4 writing); issue #1568 comment posted. E4 worktree also provisioned from old main — self-corrected to 5a035d208.
- E2 accepted out-of-scope follow-ups (→ PR body Follow-ups + maybe new issues later): -subcommands bare-name target derivation w/ lookupNsPtr; VM declared-unset namespace var storage gap; namespace unknown dispatch-time inheritance chain.
- E2 findings: which -variable has a RELEASE AXIS (9.0 TCL_NAMESPACE_ONLY, tclVar.c:5951) — pinned both releases; ensemble empty-word resolves via strncmp (documented in shared owner); E2 worktree was provisioned at cad24955f (old main!) — agent reset to 5a035d208. CHECK OTHER WORKTREES' BASE.
- Lane worktree lifecycle: implement → gates green → commit/push → report to orchestrator → adversary review (opus engine/sonnet mechanical) → fix findings → draft PR (mirror template, list "Fixes #N") → orchestrator subscribes → CI green → undraft → feedback loop → merged → cargo clean worktree.
- PR feedback routing: orchestrator subscribes via subscribe_pr_activity in MAIN session; routes events to lane agents via SendMessage.

## Wave state
- [x] R0 toolchain (agent ac16a96442b72c65a) — done incl. 8.6.16 build
- [x] R1 PR archaeology (agent a181e9cbc8d48a9de) — done, landed-map below
- [ ] R2 verifiers (launched 2026-08-18, all running):
  - V-E1 ac2f456731fd1aa9d (opus) DONE 2026-08-18: ALL 4 CONFIRMED (citations drifted, corrected). Bonus defects: tclvm int() lacks 8.6/9.0 dialect axis (#1382 scope+); tclvm errorCode NONE on 0**-1 (#1428 scope+); run_script hangs on big exponent. Lane spec saved: scratchpad/verdicts/E1.md
  - V-E2 a835112d52080d7e8 (opus) DONE 2026-08-18: #1442 PARTIAL (origin fixed; which -variable residual); #1446 CONFIRMED + NEW regression (VM export query always-empty); #1451 CONFIRMED; #1453 CONFIRMED; #1463 PARTIAL (take_command fixed by #1510, TclOO leak live via Command::Object gate short-circuit interp.rs:1193). Adjacent finding for E5: VM interp hide silently succeeds on missing cmd (interp.rs:2879-2884). Spec: scratchpad/verdicts/E2.md
  - V-E3 ae4765930bf977e87 (opus) DONE 2026-08-18: #1438 CONFIRMED (wider: +array-elem path, append/incr/lappend); #1440 PARTIAL (10a9344f7 fixed VM exec-enter; residual: runtime var-trace order, VM elem-before-array, BOTH engines cmd traces oldest-first — new); #1444 CONFIRMED all claims. Spec: scratchpad/verdicts/E3.md
  - V-E4 a7cb5ce35876e25af (opus) DONE 2026-08-18: ALL 6 CONFIRMED. #1429 repro drifted (mutating dict ops only); #1439 zero byte divergence (drift-gate enhancement); #1457 + NEW defect: VM compiled-word ${...} inverted by dialect (file new issue → E4 lane); #1458 dynamic-name path verbatim. Spec: scratchpad/verdicts/E4.md
  - V-E5 a7fc71b4b5a1eb1e3 (opus) DONE 2026-08-18: all 4 CONFIRMED. #1412 partial-landed sub-items via 7db1e39a9, item-5 C claim fabricated, expose half live; #1450 widened (+zipfs, dialect-aware needed); #1452 amplified (top-level runtime interp has NO tcl_platform at all); #1464 + 9.1 + lowercase-tcl 9.0-only. E2/E5 overlap RULED: E5 edits runtime namespace.rs rename seam only. Spec: scratchpad/verdicts/E5.md
  - V-W  ab5bc66c46a3a94ef (opus) DONE 2026-08-18: #1376 CONFIRMED (+for-step needs base field); #1459 CONFIRMED exact (keep ${} early-return on unify); #1542 STALE as filed, RE-SCOPED: link tests silently vacuous in CI (no wasmtime/wasip1/wasi-sdk installed) + unnamespaced /tmp scratch. Spec: scratchpad/verdicts/W.md
  - V-D  ab363ad993bdb467b (sonnet) DONE 2026-08-18: #1405 PARTIAL. PR #1555 (Closes never fired) landed DialectProfile/DialectSet + ingress resolution + alias canonicalisation; iRules two-spelling bug GONE (zero live string-compare sites, adversarially grepped); converted taint.rs/gvn.rs/irules_checks.rs; spec.rs citation STALE (already DialectSet-typed). Residual: tcl_expr_eval.rs 10 bare &str sites (call by_name immediately — debt not bug); 381 non-test dialect:&str/Option<&str> sites across 18 crates (bulk = LSP layer: lib.rs 29, references.rs 27, semantic_tokens.rs 24, rename.rs 14, minify.rs 13, call_hierarchy.rs 13, definition.rs 12...); closed-vocab enum items unlanded. NO live correctness bug → PR-D re-scope decision: narrow (expr_eval + compiler/registry) vs LSP-layer sweep. #1567 red herring for this issue.
  - V-G  a61bc249932f0a707 (sonnet) #1399 #1404(partial via #1554) #1407+#1416(likely stale via #1551). NOTE: initially got misrouted #1405 intel; corrected 2026-08-18.
  - V-S  ab03b95db90690e8c (sonnet) DONE 2026-08-18: #1372 CONFIRMED. tcl-engine-api/tcl-engine-tclvm/tcl-spec-hooks landed 2026-08-15 (cb2a6d739); shim unbuilt; design inputs captured in its report (fit existing tcl-engine-api Value/HostCommand shape; no string round-trips; trusted-native trust model to formalize; _Init registration via tcl-spec-hooks verb table; Tcl_WrongNumArgs/GetIndexFromObj slice first; mine docs/design/runtime/c-extension-abi.md + c-api-ownership-contract.md; crash-containment asymmetry for native code)
  - V-B  abdc6b84f3a14230c (sonnet) DONE 2026-08-18. Verdicts: #1527-#1538 ALL PARTIAL atop PR #1541 foundation (nothing landed since 2026-08-16); #1543 OPEN-UNSTARTED (no dialect bit — dialect_set.rs bits 9-12 free, SpecTcl bit-8 is the pattern; no LSP/registry wiring; shared-utility-contracts-rust.md silent on SslicTcl). Territory anchors: tcl-sslictcl/src/* (dsl.rs has only certificate/endpoint/testssl-import decls — biggest DSL gap), tcl-bigip/src/{tls.rs 1002L, validator.rs 1300L+}, xtask/src/sslictcl_data.rs (drift-gate pattern, trust-store-only scope), bigip-report-gen/. Six B1 design forks captured in its report (DSL vocab shape flat-vs-nested; CommandSpec vs DslDeclSpec; versioning/finding-id stability; policy-hook vs non-execution guarantee; #1535/#1528 merge-point owner; #1537 unified-vs-per-domain gates) → fable review needed before B2-B5.
- [ ] R3 consolidation (me): close STALE-FIXED with evidence, comment residuals, update #1473/#1524, re-cut lanes, report verdict table to user.
- [ ] PR-D #1405 — ONLY if V-D says residual work remains; else lane dissolves.
- [ ] Wave 1 lanes E1–E5, W (opus, worktrees) + Track B B1–B7 parallel. B1 design → fable review before B2–B5.
- [ ] Wave 2: G (sonnet), S (opus, fable-reviewed arch first).
- [ ] Final: perf check (sonnet), then rust/→crates/ layout PR (last, alone).

## R1 headline findings
- SYSTEMIC: "Closes #N" keywords on rust-branch PRs often did NOT close issues. Never trust PR body claims; verify issue state + HEAD.
- Likely STALE-FIXED (pending verifier confirmation): #1405 (PR #1555), #1407+#1416 (PR #1551), #1463 residual (PR #1510), #1557 (PRs #1558+#1567, out of scope anyway), #1556 (PR #1565, out of scope).
- #1404 PARTIAL: PR #1554 covered only iRule-test artefact slice; f5-query/runtime/BIG-IP artefacts remain.
- #1527–#1538: PR #1541 landed foundations only; epic tracker comment on #1524 is authoritative (12 issues intentionally open). #1525/#1526 closed.
- Engine backlog (E1–E5, W issues) untouched in window → likely CONFIRMED.
- #1473 tracker stale in one spot: #1463 parenthetical predates #1510 fix. Comment-only updates; body tables not edited.
- Owner map current; xtask owner-resolution gate live (Makefile:606, ci.yml:244).
- Tags v2.1.20 (08-18) and v2.1.21 (08-18) already cut since v2.1.19.

## Gates per implementation lane
tee to /tmp/, grep the log: make prep-pr; make test-rust; cargo test -p tcl-lsp-server; make test-ext; engine lanes add make runtime-rust-test. Adversary review BEFORE draft PR opens. Draft PR mirrors template → subscribe_pr_activity → CI green → undraft → drive to merge.

## Scope exclusions
#1181, #1363, #1556, #1557, #1560 untouched. #1473/#1524 = trackers (bookkeeping only).

## 2026-08-20 08:15Z — RESTART #6 = FILESYSTEM ROLLBACK (state file below this line rebuilt; older content above is a stale snapshot)
- The restart restored an OLD fs snapshot: three long-deleted merged-lane worktrees returned with 26G of stale targets (disk hit 100%/128M free — deleted again → 26G free); F5/F6 worktrees GONE (F5's two unpushed commits + F6's uncommitted investigation LOST); local git refs stale (fetch brought origin/rust 5a035d208→580c19033); this state file reverted to 71 lines. GitHub is the source of truth and is fully intact.
- SESSION TRUTH (rebuilt): 19 PRs merged (#1587 #1591 #1592 #1596 #1615 #1618 #1621 #1622 #1630 #1632 #1636 #1640 #1639 #1641 #1642 #1645 #1647 #1649 #1652), 48 issues closed. rust @580c190332a. Open tracked issues filed by me recently: #1643 (post-release), #1644 (pre-release), #1646 (post-release), #1648 (post-release), #1650 (JetBrains half of #1626, from F3), #1651 #1653 #1654 (pre-release), #1655 #1656 (see labels).
- NEW DOCTRINE (rollback lesson): lanes must PUSH EARLY AND OFTEN — after each locally-green unit — because rollbacks destroy unpushed work; both respawn briefs carry this.
- RESPAWNED FRESH: F5v2 (a700ac5a27dd6571e) → #1653+#1654, branch claude/f5-fixes-1653-1654, brief carries lost incarnation's fix titles (stub-defaulted-metaclass-not-an-observation; depth-caps-sized-to-real-stack-budget with #996 big-stacks pointer). F6v2 (a71715b19b9f07287) → #1600+#1651, branch claude/f6-fixes-1600-1651, brief carries marker-vs-publish channel evidence + #1622 lock-table + #1649 cascade precedent + #1647 clamp idiom.
- Queue after F5v2/F6v2: #1624 #1606 #1608 #1617; E1 finish low priority (its worktree also rolled back/gone — branch claude/e1-expr-numbers@a8058849d exists on REMOTE, verify before assigning).
- Old agent IDs from before the rollback are DEAD (transcripts wiped): do not SendMessage a8*, ab1*, a03*, a24*, ae2e*, af30*.

## 2026-08-20 — USER DIRECTIVE (standing, highest priority among disk rules)
- "As disk runs low always check in everything in the worktrees to ensure nothing gets lost."
- Operationalised: the moment ANY low-disk signal appears (df-gate trip, escalation ladder, cleanup pass, ENOSPC), step 1 — BEFORE any deletion or build abort — is: in every live worktree, `git add` the lane's real work (never `git add -A`; the tmp symlink must stay untracked), commit as wip if needed, and PUSH the branch to origin. Only then clean targets/artifacts. Orchestrator enforces this in every disk sweep; every lane brief carries it from now on. This supersedes "clean first" ordering everywhere.
- USER DIRECTIVE addendum: "put any context required for continuing in a file in the repo or as a comment on the issue." Operationalised: at every durability checkpoint (low-disk signal, before long builds, at each push, when reporting to orchestrator), a lane must persist its CONTINUATION CONTEXT — current position, verified facts, evidence, next steps, open questions — somewhere that survives container loss: preferred = a progress comment on the assigned issue; alternative = a LANE-NOTES.md committed on the lane branch (dropped in the final tidy before the PR is review-ready, or folded into the PR body). Scratchpad and transcripts do NOT count — both have been wiped by rollbacks. Orchestrator: my own state file is also at risk → mirror critical orchestration state (lane map, queue, standing rules) into issue comments or a pushed branch file at major transitions.

## 2026-08-20 09:11Z — check-in
- ROLLBACK DOCTRINE WORKING: both v2 lanes pushed early — claude/f5-fixes-1653-1654 @901bb5c0c, claude/f6-fixes-1600-1651 @758ac0512 on origin. No PRs yet (~1h in, normal). Only their two worktrees present (no zombies). Disk 18G.
- Labelling: no new issues since #1656; latest five all correctly labelled. Compliant.
- Queue unchanged: #1624 #1606 #1608 #1617 after v2 lanes; E1 finish low priority.

## 2026-08-20 10:08Z — check-in (light)
- Both v2 lanes actively pushing: F5 @1024a7032, F6 @5c8088a20 (advanced since 09:11). No PRs yet (~2h in). Branch movement = health signal, no pings.
- Disk 7.9G (builds in flight), above gate. No zombie worktrees. Labelling compliant (no new issues).

## 2026-08-20 10:15Z — F6v2 delivered
- F6v2 DONE (pending review): PR #1658 open+subscribed (Fixes #1600 #1651). FINDINGS OVERTURNED TWO PRIORS: (a) marker-vs-publish channel hypothesis WRONG — publish precedes marker on stdio_pump's single FIFO; real #1651 mechanism = schedule_diagnostics reusing DiagSlot::latest_inputs while run_config_reload applies switches THEN walks disk for packs — edits in that window analyse under pre-apply config; pack-walk width (load/page-cache) is the local-vs-CI divergence; USER-VISIBLE, not test-only. Fixed via diag_inputs_epoch invalidation from every state-writing path. (b) "wedges are always cascades" (F3's finding for its case) NOT universal — a GENUINE wedge reproduced: didOpen drain unanswered 101654ms, starvation ruled out, document-free probe dead → filed #1657 (bug+pre-release, correctly NOT forced into this PR). Probe fix: liveness getEffectiveConfig now passes no URI (was taking EditOrder barrier + documents lock + db mutex — the very locks a stall holds); latch now requires all three probes unanswered.
- #1658 evidence: baseline 5 runs → 1 repro with verbatim signature; fixed 7 runs → 0 (one at loadFactor 6.8). Ruling on its flagged deviation: KEEP "Fixes #1600" — its asks are delivered; #1657 is the successor. One unrelated e2e flake noted (converges-via-refresh, passed on re-run + clean pass).
- #1657 candidate mechanisms: DeferredConcurrency 4-permit pool starved by handlers parked in edits_settled holding permits; or a lost EditOrder ticket (future dropped while waiting never advances now_serving → permanent wedge). NEXT ASSIGNMENT for F6v2 after #1658 merges — it owns all the context.
- Awaiting CI+Codex on #1658 → merge, close #1600 #1651.
- 10:17Z: Codex 3 findings on #1658 routed to F6v2: (1) P2 epoch TOCTOU — resolved snapshot can commit with stale epoch / clobber a newer N+1 commit, worker never revalidates → recheck-after-resolve + interleaved tests + mutation; (2) P2 /proc/pid/io rchar/wchar are process-aggregate not transport-specific → relabel evidence honestly (fd-level option noted in #1657); (3) P1 KCS terminology rule (AGENTS.md L594-596) — define/link pid/CPU/stdin/mutex per house pattern. Merge held.

## 2026-08-20 10:28Z — F5v2 delivered
- F5v2 DONE (pending review): PR #1659 open+subscribed (Fixes #1653 #1654), two clean commits, everything pushed, worktree clean, 24G free.
- #1653: ClassDef::metaclass_provenance (StandIn/Observed) gates the join; fixing it EXPOSED a masked second hole — winner-take-all join dropped the loser's members (invisible before because the mismatch abstained onto the stub, which kept them) → ClassDef::absorb_declarations (exhaustively destructured so new fields must be classified to compile); collision rule oracle-backed (oo::define side wins per tclsh 8.6.16).
- #1654: issue's supposition WRONG — the generic analyser walk has a bound; the aborting walk is LOWERING (no analyser frame in the gdb backtrace). Stack probes: lowering 18,864 B/level vs cfg_builder 8,288 vs analyse_body 3,840; abort at ~112 levels with cap at 256. Three caps now derive from depth_guard::MAX_SOURCE_NEST_DEPTH = (2MiB − quarter reserve)/20KiB. Production uses #996's 64MiB stacks; sizing to the floor protects other callers. DECLARED behaviour change: cap 256→lower (review-notes flagged).
- Mutation 18: 17 killed + 1 proven equivalent (oracle-backed); 5 first-pass survivors drove 5 extra tests. Gates green incl. #996 e2e at 140/150/500/2000 levels; commit-1-alone check (bisectable). tcl-spec-studio skipped with justification (no registry-visible surface).
- F5v2's not-fixed finding → I filed #1660 (bug+compiler+pre-release): post-pass-proved metaclass can't classify same-file creation call (deferred-verdict shape suggested).
- Awaiting Codex+CI on #1659 → merge, close #1653 #1654. #1658 (F6v2) in Codex round concurrently.
- 10:30Z: Codex 4 P2s on #1659 routed to F5v2. Theme (3 of 4): absorb_declarations honours define-wins for methods only — visibility sets (export/unexport last-writer, clear opposite), ordered relation slots (mixin REPLACES unless -append — oracle to confirm), tombstones (deletemethod retraction must apply to the joined member table, appending isn't enough since compiler consumers never run the workspace retraction fold). Plus: stack-budget regression test drives only analyse, not LOWERING whose 18.9KB/level sized the cap — must drive lowering+CFG and prove it bites via artificial under-sizing. One oracle session settles the three semantics. Merge held. Both PRs now in concurrent Codex-fix rounds.
- 10:40Z: test-ext RED on #1659 @2705ca2b6 — the #1657 WEDGE, verbatim signature, on CI at load factor 1.0 (3 tests: didOpen drain 112s, starvation ruled out, all three liveness probes dead; pack-association suites immediately preceding again). #1659's diff is compiler-only → structurally exonerated → failed jobs re-run queued. ESCALATION posted on #1657: wedge no longer needs load (favours lost-EditOrder-ticket over permit starvation); now taxes every PR's test-ext. PLAN: assign #1657 to F6v2 immediately after its #1658 Codex round lands — it owns the capture + context. If the re-run wedges again, #1659 merges may need to wait on the #1657 fix instead of re-run roulette.
- 10:47Z: pr-gate red on #1659's NEW head 4d51b5e90 (F5v2's Codex-round push): clippy too_many_lines in tcl-compiler — pre-push gate miss. Routed back (extract or justified allow, full gate, push). test-ext rerun on old head superseded by the new push anyway.

## 2026-08-20 11:09Z — check-in (light)
- Both lanes pushed post-brief: F5 @cbd4dd705 (clippy fix), F6 @421d22b1e (Codex round: epoch TOCTOU + io-relabel + KCS terms). CI pending on both; events drive merges. Disk 15G. Labelling compliant. Queue unchanged (#1624 #1606 #1608 #1617; E1 low; #1657 reserved for F6v2 next).
