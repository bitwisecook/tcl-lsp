# perf agent: final source and evidence handoff

Last updated: 2026-10-24 03:22 UTC

Six owned files landed in **9ffdce1364f286450499deaf915a037deea28117** on `work`, as one source commit, with no
old/new switch and no realm/interprocedural production changes. Latest-work
metadata checks for lexer/compiler/SpecTcl/server test targets pass with zero
warnings/errors; scoped and materialized-workspace formatting pass. The initial
latest check failed only missing tracked sparse fixtures; exact canonical
materialization and the successful repeat are both retained.

perf-realm-interprocedural.patch proven at 83774d43464c7d177bcd505ef22fe898ffee9a25, ready for the compiler agent

Proof scope: frozen **83774d43464c7d177bcd505ef22fe898ffee9a25** plus the archived full batch and switch,
Rust-diff SHA-256 `8d34b06306190679c0de73d223bc3c33accec4f428a3e56d5e17d21c1e57949e`. Every one of102 UTF-8 Tcl files,15 iRules files
and7 synthetic sizes passes all3 complete authenticated-report comparisons in
BOTH forced-old and mixed runs. No Different/Unavailable, omitted field,
normalization or sorted rendering. Strict runtime/global/cache equality stays
unchanged and is separately recorded. Ordered complete PackSet Debug agrees in
all3 directions,484936 lines,8 packs/0 notices/0 load errors, deterministic old
endpoints. Original native0xff remains a separate passing compiler control.
This bounded equivalence proof does **not** make the failed gates below green.

The two-file patch refreshes against the landed six-file commit. Clean integration
also needs **perf-routed-clean-only-helper.patch**, a one-line `cfg(test)` on the
old binding wrapper after its production callers move to the realm API. The same
helper and every assertion remain in its existing unit tests; no production query
or memo-selection policy changes. Apply the companion only with the routed two
files, never to a six-only or switched tree. The old one-warning build and the
warning-free compiler/server lib+test metadata repeat are preserved in
`perf/evidence/landed-six-routed-typecheck-9ffdce13/`. Perf explicitly routes this single owned-file
integration line to the compiler agent together with the two-file patch.
`perf.patch` includes all8 clean files and this companion; `perf-owned.patch`
continues to describe only the independently buildable six-file commit.

## Runtime and lint results

| Check | New | Old / comparison |
|---|---|---|
| Authenticated report/actual-realm controls |22 pass|22 pass|
| Switched lexer lib |586 pass|586 pass|
| Clean six-file lexer lib |586 pass|586 pass, ENV ignored|
| Full compiler nondeep filters +native0xff +ReadFlow |143 pass/89 fail,232 complete|identical, zero P2F|
| Clean six-file nondeep filters +ReadFlow |142 pass/89 fail,231 complete|identical BOTH and to actual switched OLD baseline, zero P2F|
| Full unchanged depth1000 ALONE |OOM437.385s,16134540KiB peak|OOM449.877s,16037596KiB|
| Clean six unchanged depth1000 ALONE |OOM440.235s,16320104KiB|OOM371.861s,16298828KiB|
| SpecTcl eval_loader |28 pass,319.619s|28 pass,257.425s|
| SpecTcl golden_packs |3 pass,314.814s|3 pass,308.789s|
| Quiet server startup/reindex |first role assertion0vs1,3.952s|same,3.831s; later factory assertions unreached|
| Quiet workspace_symbol cap |timeout180.020s|timeout180.026s|
| Scoped fmt |pass|latest materialized-workspace fmt also passes|
| Compiler all-target Clippy |56 errors|0 in owned6/realm/interprocedural; API-file error is an unchanged pre-repair function|

All actual baseline-positive outcomes are retained; zero observed pass-to-fail.
Each depth1000 process exits-9 with0/1 Rust outcomes, so both strict complete
compiler-inventory assertions **fail**. No kill is fabricated as a Rust test
result. Server gates still fail in both modes on the quiet host. Clippy remains
failed on56 retained diagnostics outside the perf changes; the single repaired-
API-file function is byte-identical before/after ce6facfb. No suppression,
assertion, depth, stack size or timeout was weakened. See
`perf/compiler-resource-routing.md` and `perf/residual-routing.md`.

All runtime builds have0Rust warnings. Compiler/control/client executables and
private copies were removed by their runners, and source seals were checked.
Profiles: opt0 for lexer/compiler/server; opt1 for report corpus/SpecTcl;
debug0, incremental0, debug/overflow assertions on. Required Tcl9.0.4/8.6.18
and Jim0.84-9-g5bac7c9 remain pinned. Targets are separate per root.

## Measured full-batch timings

Milliseconds, opt1 wall observations; report capture included for analysis,
comparison excluded. These measure the full8 batch, including the routed two
files, **not** the landed six alone. They are not release-latency or linearity
claims. Full results: `perf/evidence/current-83774d43-complete-report-summary.json`.

| Input | old1 | new | old2 |
|---|---:|---:|---:|
| calls10 |1175|565|753|
| calls20 |1999|929|2529|
| calls40 |5690|1674|4915|
| calls80 |20346|3726|19408|
| procs41 |6311|11869|2645|
| upvar10 |7180|3597|6293|
| upvar30 |58873|17970|46069|
| load_embedded, ordinary API |14302|4068|11116|

The slower procs41 result is retained. Exact-input/source repeats also compare
Equal:2831/5324/2268ms and2184/1379/2538ms; timings vary in both directions.
Origin/rust fcb7927f same-input timings and all input-byte audits are complete:
117 text inputs,7 emitted synthetics and8 SpecTcl sources. The original probe
read work-checkout fixture paths, including19 absent from the origin checkout.
Origin plain analyse excludes authenticated capture, so no cross-boundary ratio
is claimed. Origin ordinary load_embedded4665ms is a separate same-API
observation. Historical tclIndex253 old5108792/new189404/origin1596ms and ordinary
LSP80 index1557.801ms/lenses25542.691ms remain explicitly scoped to their earlier
source/profile in `perf/evidence/timings.md`; no fresh tclIndex253 claim.

## Remaining owner work

Compiler integrates the refreshed two-file patch plus its clean-only helper
companion, retaining every current owner/source/config/registry guard and the
later canonical ReadFlow repair0f4b8e19. Current type checks are separately
scoped to9ffdce13 plus those patches; no successor full runtime proof is claimed.

Residual image/segment, nested LSP inventory and empty-region drafts refresh to
the landed commit and remain **Blocked/routed, uncompiled and unproven**.
Compiler-names owns immutable operation admission/image/segmentation reuse;
LSP owns shared request inventory/occurrence work; compiler owns body/graph
resource behavior. The image draft still scans full bytes per reuse and the
LSP draft retains per-lens outer capture/walks. Review4.1 ordinary-path scaling,
startup role withdrawal, workspace-folder/WASI exit behavior and the resource
failures are not fixed by this handoff. Owners must prove complete outputs and
positive ordinary-path counts before accepting those drafts.

No PR or force push. Historical failed/partial runs remain in the evidence tree;
this file describes final scope rather than replacing those results.
