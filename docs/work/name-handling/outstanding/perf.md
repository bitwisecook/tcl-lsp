# Outstanding work: perf agent

Area: Analysis and SpecTcl pack-load performance.

Last updated: 2026-10-24 03:22 UTC

## Resume

**Perf six-file source change is LANDED: 9ffdce1364f286450499deaf915a037deea28117.** One source commit,
exactly the six requested files, no realm/interprocedural hunks or switch.
Do not restart the completed long proof jobs. Final results and timing scope:
[perf-NOTES.md](../handoff/reviews/merge-inventory-20261019/perf-NOTES.md).
No PR or force push. Attribution: Co-Authored-By: Codex <noreply@openai.com>.

perf-realm-interprocedural.patch proven at 83774d43464c7d177bcd505ef22fe898ffee9a25, ready for the compiler agent

Readiness is the bounded full-report/PackSet equivalence proof below, with all
baseline failures retained. It is not complete compiler/server/Clippy gate
acceptance. The two-file artifact refreshes against9ffdce13 and needs the
separate **perf-routed-clean-only-helper.patch** at integration: cfg(test) keeps
the now test-only old wrapper for its unchanged tests. Apply only with the two
routed files and without --zz. Six-only source still needs its production
wrapper; switched proof source still needs the old path. Perf routes that single
owned-file integration line to compiler. Clean compiler/server lib+test metadata
check passes0warnings/errors; no production query, guard or test assertion changes.

## Completed proofs

- Frozen source83774d43464c7d177bcd505ef22fe898ffee9a25 +full8+switch; exact switched diff
  `8d34b06306190679c0de73d223bc3c33accec4f428a3e56d5e17d21c1e57949e` is archived in current-83774d43-repaired-workflow-api22/.
- All22 authenticated report/actual-realm controls PASS BOTH,0warnings.
- Complete102 UTF-8 Tcl+15iRules+7 synthetic sizes, all3 complete report pairs
  Equal BOTH forced-old/mixed. No Different/Unavailable, input or field dropped.
  Strict runtime/global/cache Eq remains strict/separately recorded.
- Complete ordered PackSet484936lines/8packs/0notices/errors, all3 equal and
  old endpoints deterministic. No sorting/normalization.
- Switched and six-only lexer586 PASS BOTH,0warnings/executables removed.
- Full232 nondeep outcomes identical143pass/89fail BOTH. Six-only231 outcomes
  identical142pass/89fail BOTH AND equal every actual switched OLD baseline
  outcome. Extra full case is passing native0xff. Zero observed P2F.
- SpecTcl eval_loader28 and golden_packs3 PASS BOTH,0warnings/source exact/
  executables removed. Final quiet server attempts complete; failures below.
- Latest-work six-only lexer/compiler/SpecTcl/server --tests metadata and scoped/
  materialized-workspace fmt PASS0warnings/errors at75768f36. Earlier absent
  sparse fixture failure and canonical materialization audit retained.
- Same eight unpatched target bytes at proof base and landed parentdb3267eed849a59df3ab2973d83f43f20df0ad1a;
  six candidate source bytes unchanged. Current clean full8 plus helper companion
  types under compiler/server --lib --tests at9ffdce13 with0warnings/errors.
- Origin fcb7927f timings/audits complete for the same117 text+7 synthetic inputs
  and8 SpecTcl sources. Full provenance in timings.md; plain origin analyse
  excludes report capture, so no cross-API ratio claimed.

## Blocks routed to owners

1. Compiler/compiler-names: all4 unchanged depth1000 attempts OOM ALONE,
   exit-9/0of1 Rust outcomes each. Full437.385/449.877s; six-only440.235/371.861s.
   Strict complete-inventory assertions FAIL. No fabricated Rust result.
   Resource summary and compiler-resource-routing.md preserve exact peaks/raw
   outputs/cleanup. Original/private test executables removed; source seals exact.
2. LSP/compiler: quiet startup still0vs1 role assertion BOTH; later factory
   reindex assertions unreached. Workspace-symbol still180.020/180.026s timeout
   BOTH, no new P2F. Broader workspace-folder/WASI behavior not accepted as fixed.
3. Clippy:56 errors retained; zero in six owned/routed2. The API-file diagnostic
   procedure function predates ce6facfb byte-for-byte. Full gate remains FAILED;
   no suppressions. Canonical ReadFlow0f4b8e19 must be retained; its earlier837
   conformance failure is preserved, not relabelled green.
4. Compiler-names/LSP/compiler: refreshed residual image/segment, nested request
   inventory and empty checked-region drafts remain Blocked/routed, uncompiled
   and semantically unproven. Full-buffer checking and per-lens outer rebuilds
   remain; no linear review4.1 or deep-OOM fix is claimed. Owners prove full
   results/PackSets and positive real ordinary-path counts before integration.

## Evidence and state

Publisher:/workspace/tcl-lsp-publish, branch remote work. Writer:/workspace/
tcl-lsp-work supplies evidence only; its old product source is not proof source.
Frozen full proof:/workspace/tcl-lsp-diagnostic at837+9, never commit switches.
All runtime and lint helpers finished; compiler/client executables removed.
Clean integration proof:/workspace/tcl-lsp-owned at9ffdce13 plus the two routed
files and clean-only one-line helper companion, never commit those local hunks.
Metadata checks create no test executable; source diffs are sealed.

Main evidence:current-83774d43-repaired-workflow-api22/, current-83774d43-clean6-
workflow-api22/, current-83774d43-complete-report-summary.json, current-83774d43-
resource-summary.json, latest-work-six-file-check-75768f36/ (failed sparse attempt
and successful materialized-inputs-retry), landed-six-routed-typecheck-9ffdce13/
(one-warning attempt and successful clean-only-test-helper-retry).

Env:CARGO_INCREMENTAL=0 CARGO_PROFILE_TEST_DEBUG=0 TCL_LSP_REQUIRE_TCLSH=9.0,8.6
TCL_LSP_JIMSH=$PWD/tmp/jimtcl-oracle/jimsh; debug/overflow assertions remain on.
No ENOSPC; all four actual depth OOMs retained. Every push fetches/rebases latest
work, retries normally on rejection and never forces. Final notes updated; perf
stops after publishing this handoff. Remaining work is explicitly routed above.

## Historical coordinator delivery records

### Coordinator routing — 2026-10-22T22:56:21.361198+00:00 (owner header unchanged)

The user authorized **authenticated full-report comparison** as the new perf equivalence acceptance contract. Decision: `../handoff/reviews/resume-20261017-diagnosis/user-decisions-20261022T2256Z.json`. Compiler owns the distinct API and authentic native/hosted carriers; global/runtime equality and cache/Salsa cutoff stay strict. Every field and typed subject requires complete unique current actual correspondence; Unavailable fails the gate. TEMP3 current-source attribution is in `../handoff/reviews/merge-inventory-20261019/compiler/resume-20261022/report-first-refusal-20261022T221949Z/`. Resume harness/full old+mixed corpus work when the reviewed API/carriers arrive. No perf patch, including realm/interprocedural, is accepted or forwarded by this decision alone. No duplicate perf role started.

### Coordinator dependency delivery — 2026-10-23T01:05:06.334470+00:00 (owner header unchanged)

Compiler has frozen the user2256 report-only API proposal v7, patch `2351e4cf48adf9d66ddfaa75661e01e4ffc8bc405b4c61e6f1bac4e3dc5cbc8b`, with bounded naming peer review `b52aea24`. [Exact API and 14-control handoff](../handoff/reviews/merge-inventory-20261019/compiler/resume-20261022/authenticated-report-v7-20261023T0101Z/) and [guarded execution preparation](../handoff/reviews/merge-inventory-20261019/compiler/resume-20261022/authenticated-report-focused14-v1-20261023T0106Z/) are preparation only. Canonical product does not yet include this API; no typing or control/corpus run is claimed. Existing perf session can prepare the harness off-tree from the frozen signatures; wait for root publication and successful controls before actual full all-old/mixed proof. Equal alone passes; Different and Unavailable fail the proof. Keep strict global/runtime equality and cache cutoffs separately recorded, complete source/input/registry/body/hosted authentication and count guards unchanged. Realm/interprocedural patch remains withheld until this owner records it proven. Root has not duplicated or taken over perf. Filename times are labels; see actual receipt timestamps.

Current procs41 timing follow-up: same opt1 source/input, two further full
old/new/old comparisons Equal:2831/5324/2268ms and2184/1379/2538ms. Retain
the original6311/11869/2645ms too; variable timings do not support a universal
speedup claim.0warnings/source exact/probe source+ELFs removed. Evidence
current-83774d43-procs-timing-repeat/. Original origin probes are removed; exact
sources/hashes preserved in origin-temporary-probe-cleanup/. No live job touched.

Final server quiet repeat is BUILDING on frozen837 full9, all4CPUs and no
other builds/tests. Both compiler masters finished/cleaned. Only after its
original2cases/BOTH180s and cleanup, final scoped fmt/alltarget Clippy runs.
No six-file source commit or two-file readiness claim yet.
