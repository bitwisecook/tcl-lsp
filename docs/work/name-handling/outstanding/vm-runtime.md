# Outstanding work: vm-runtime agent

Area: tcl-vm, runtime/rust, tcl-spectcl, tcl-spec-studio: SpecTcl execution and acceptance.

Last updated: 2026-10-22T19:23:42Z (partial host N13 purpose/context review complete; one context limitation sent LSP; cfg(test) cleanup root-approved held; no VM Cargo queued)

Maintained by the vm-runtime agent. It lists what this agent considers it must still complete. Update it whenever an item starts, finishes, changes or becomes blocked, and at least every 30 minutes while working. The coordinator commits and pushes it with each WIP snapshot.

## Resume

Active tree is `/workspace/naming-migration-active/tcl-lsp`. Root reports the
257-path coordinated batch applied, including VM47 lint and ASCII3 backend.
Trusted Jim proc descriptor `346d63ea` and the VM callback signature fixes are
now root-applied (coordinator evidence
`typecheck-r2-and-semantic-fixes-20261022T1819Z`). R2 closed with stable source,
zero warnings and four errors total: two LSP Core ownership/lifetime errors and
two VM test callback-generality errors. The production callback correction
`a2330cb9` had cleared those production errors; the analogous test-only
`8f0c5107` correction is applied for R3. Callback bodies, currentness checks,
mutation/reference-count assertions and TLS retirement guards are unchanged.
Root reports Jim11, the explicit D4 child-module path, server lint and three R3
fixes integrated in `d666c9b2a7142a0f1e5940e81681efef769d7fe2`. The D4
E0583 module registration error has a root-applied source correction; no test
pass is inferred. Current compiler18 guarded execution has GO, followed by
current LSP3 then VM11 then CCG N17. Canonical Rust/tests remain frozen; docs
are explicitly clear. No fresh completed seven-crate/type-gate claim is made here.

No VM-runtime Cargo process has entered or queued. After type clearance, root
orders original compiler18 selectors, then short original LSP3 guarded-loss
diagnosis, then VM11/SpecTcl acceptance. No slot is released to VM yet. Exact
staged VM commands/selectors/guards are in
`/tmp/vm-runtime-preflight/execution-selector-plan.json`: six after8b Jim/EDA
targets (11+4+3+1+8+5 = 32 original cases), then full `jim_document` with the
original26 outcomes retained separately from added controls, TLS2, exact owned
six-vendor EDA smoke/D4 and one explicit LSP clean-exit recovery selector. Each
whole preguard/build/artifact-hash/run/postguard stays under the global build
flock; pinned source/provider hashes, exact nonzero rosters, no ignored tests,
zero warnings and cap/truncation fail-closed checks are required. General VM
parity and unfiltered library/e2e runs remain parked. LSP confirms no duplicate
watchdog/TLS/EDA/spec-pack selector queued.

C.1–C.2 acceptance mapping is prepared off-tree at
`/tmp/vm-runtime-c1-c2-acceptance-map.json` with exact source rosters/hashes and
build/run templates for 25 additional target groups. This is scheduling and
coverage analysis, not execution or new authorized jobs. The ready11 jobs cover
32 after8b surface cases, Jim34, TLS2 and three exact server selectors (71 total
expected tests after applied Jim controls). They do not close the mandatory
five-target compatibility gate, all17 UPF executable slots, Jim10 final protocol,
remaining30 parent SpecTcl protocol cases, hook-worker generation controls,
trust/roster/environment reload checks, studio round trips or host delivery.

CCG reserves its exact51 selected cases: registry ASCII3, variable-reference
boundaries4 (including exact N17 good/braced controls), pack_source original6
plus ASCII4, const_fold6 and codegen_stamps28 of29 (one heavy case excluded).
VM will not duplicate those five target groups. The independent compiler
`compile_service::tests::a_variable_reference_ends_where_the_release_scanner_ends_it`
is not in compiler18 or CCG51; root must schedule or reconcile it separately.
Remaining mandatory compatibility members are eval_loader28, golden_packs3,
spec_corpus5 and exact9.0 parse-only1; CCG owns pack_source10. Split plans alone
are not a successful `make test-spectcl-compat` result.

UPF `upf_hooks` has two tests exercising all16 optionarity bodies plus the one
argument-role body with expected answers; it is outside ready11 and unrun here.
Native/embedded and MemoryStore delivery controls are mapped separately from
actual browser/WASI transport, which remains root D6. D4 publication receipts
and warm/fresh first answers do not establish future transport silence; the
controlled backend stale-worker proof remains a complementary generic-server
check. Cold/warm/first-worker/reload-memory measurements remain performance-role
and root release evidence. Old Rust successes/failures and finite native/Jim
manual oracle receipts are explicitly separate, with the Tcl8.4 NUL failure
preserved. No coverage closure, budget increase or general VM parity is claimed.

Partial N13 stage2 static peer review is recorded at
`/tmp/vm-runtime-n13-stage2-peer-review.json`, for exact draft
`33bdf6c763aeed82d35071d59eb91681b3167cb8ddbb276856a6aa3c7f24d3f2`.
Two concrete gaps were sent to names/LSP/root: inherited `child_state` does not
compare all requested realm/policy/dialect/Logical/Vendor/scope axes to the
minted point while root processing still consumes raw requested realm; existing
public `lower_procedure_target` can retain a point then stamp Native
`ProcedureCode`. No downstream opcode admission or runtime exploit is claimed.
Names acknowledged both and keeps stage2 frozen, preparing full axis comparison
and explicit fail-closed lowering controls in a successor. Exact parent/source/
config/owner and graph-point joins, transitive ancestor checks and terminal
invalid-state refusal were reviewed without a concrete static issue found in
those paths. This is not whole-scope approval: host publication/currentness,
all-ancestor retirement, two-parent/twelve-hop convergence, complete ingress
purpose audit and all type/functional checks remain partial or unrun. No VM
implementation, source application or Cargo execution occurred.

Frozen first-edge host N13 review is retained at
`/tmp/vm-runtime-n13-host-v1-peer-review.json`, exact host snapshot SHA-256
`4b1db55db7eaa7cc9058a20f1b5736f8a978127481ad811a87775fffccd2f7ac`.
No Native Loaded/entry/argument/frame/handle manufacture was identified in this
source-only host path. Retained parent/point and all-ancestor live table,
publication, source/config/provider checks preserve the intended static currency
boundary. One concrete context limitation is sent to LSP/root: physical child
ordinary-document input/config is also used as inherited producer input, while
compiler point ingress requires exact parent input. Different parent/child
contexts therefore refuse, and distinct parent inputs cannot coexist for one
physical child without separating physical currency from interpretation input.
This is static partial-scope analysis, not a measured regression or whole approval.
HandlerSelection, FileReadAndDecode and ContainingBody remain applicability debts;
ordinary editor UTF-8/CR-normalized images cannot become Native Loaded receipts.
Names' exact two-word/path0 issuer restriction remains required; `-encoding` has
no typed decoder proof. Parent prefix/tail continuity and host reconciliation
remain unresolved. CCG independently reviews wire/currency.

The current compiler build reported unused production helper
`OriginalClassDefinerSource::native_argument_input`. Whole-compiler `rg` found
only its definition and the original cfg(test) call at line516, so the tiny
cleanup gates that helper with `#[cfg(test)]`, preserving every test assertion.
Off-tree patch `/tmp/vm-runtime-unused-native-argument-input/test-only-helper.patch`,
SHA-256 `a34c9b61752f06cd0c25f2e7575c8e8a99b5ca518c5c1f87b15b047e3464e0f8`,
has before/after hash receipt and root static approval. It remains unapplied for
the next coordinated source window; no allows, Rust writes or Cargo here.

Trusted Jim proc attachment has compiler static approval, including exact
trusted compiled-in boundary, selected row and pointer/generation idempotence.
Direct helper negatives preserve ordinary non-CoreJim and existing-explicit
rows; root applied the two-path patch. It remains semantically unmeasured here:
no Jim26 pass count is inferred from static review or typechecking.

The four Jim class-list failures have a concrete static source boundary:
`source_class_declaration_attempt` admits `command_installation` (Snit/Itcl) or
TclOO, excluding JimClass. `OriginalClassDefinerSource` also requires all-static
arguments and cannot carry computed bases/dictionaries. The separate complete
source-only candidate is statically approved and root reports it applied in
`d666c9b2`; retained off-tree exact patch:
`/tmp/vm-runtime-jim-class-shadow/whole/jim-class-whole.patch`, SHA-256
`9c97198bc9297b4eee9cea8af768a53732d456d37f1ba7202aeb6955d8926759`
(eleven paths). Names statically approved the prior policy/rename/replacement
boundary; compiler final static approval covers the exact current eleven-path
SHA, including independent body catalogue/effects separation. Current source
contains all original26 plus eight added controls, static roster34. No VM-owned
compile/run has executed; the original26 outcomes remain separately required.

Distinct source purposes retain actual authored Jim0.84 recipe versus a
positively retained older Logical full input/context/config. Original lists
and actual parent source bodies stay owned; Native classes, arguments, bodies,
frames, receivers and publications are not issued. Parent-captured base
providers join unique original declaration/dictionary spans rather than report
names. Computed/odd replacements retire earlier provider cells; renamed
providers retain original producer lineage. The independent readonly body
catalogue records Jim rows through `authored_prefixes` without enabling general
execution/mutation-completeness inventory. Nested base relations require the
same exact parent body. Eight fixed controls retain all four original layout
cases plus foreign/stale/absent policy, rename/replacement, expansion and nested
namespace/if/procedure cases across both source purposes. Existing Jim26
expectations remain unchanged. Parse/application checks pass with exact hashes;
there is no class/inheritance acceptance claim. The pinned Jim oracle's25
fixed semantic controls remain evidence about Jim semantics only.

Independent ILX source-owner peer review is complete at
`/tmp/vm-runtime-ilx-peer-review.json` for exact compiler draft `b82481c0`.
Integration is held: its quiet predicate omits declared source continuation and
full effect-domain/interpreter/reentrancy closure; concrete break/continue rows
have empty effects yet transfer control. Current ILX init/call/notify rows lack
a declared world-effect chain, so conservative UNKNOWN currently withholds
positive admission. No EMPTY/quiet grant is inferred from remote_method or
missing executable capability. Authentic Vendor body/home/original-word and
selected descriptor/family boundaries plus named-write-before-install look
sound; no Native HandleScope/key donation exists. Compiler agrees with the hold.
No implementation, application, Cargo or duplicate BIGIP consumer work occurred.

Root reported the D4 module registration E0583 during R3 and will add the
explicit child path attribute in its next guarded source window. The child file
exists; assertions and exact selector stay unchanged. This prerequisite is
recorded in the VM selector plan; VM does not self-edit during the source hold.

Six-vendor EDA smoke is prepared in `spec_packs.rs` and included in the WIP, with
all eight pack names, own-vendor plus shared SDC/UPF availability and rival-vendor
exclusion checked in one server process. It is unmeasured; section D stays open.

Accepted D4 protocol case is now implemented as the child module
`rust/tcl-lsp-server/tests/e2e/spec_packs/environment_lifecycle.rs`, linked from
the owned `spec_packs.rs`. It changes folder configuration, the workspace-wide
custom pack and dialect sidecars through change/remove/restore while all three
open document versions remain 1 and their bytes plus the unopened provider stay
exactly fixed. It compares first warm/fresh naming, callback, class, signature,
role and live-hook answers. Readiness uses current configuration/publication
and raw read-only `currentDiagnostics`; exact old/new identities are checked
through `validateDiagnosticPublication` before and after the fixed provider
sequence. No naming-answer polling, new sleeps or production instrumentation.
LSP owns the complementary controlled stale-worker completion regression;
transport silence is not a retirement proof. Rustfmt parses/formats the test;
it has not compiled or run, and grants no section D admission.

New release blocker: external strict Clippy on `aeecf39743b` with Rust 1.99.1
reported **379 VM diagnostics across 72 paths**. Inventory:
`/tmp/naming-clippy-vm-runtime.json`; canonical external receipt is
`../handoff/reviews/merge-inventory-20261019/bigip-lsp/clippy-dependency-blockers-20261022.json`.
These are old-snapshot observations, not a current source measurement. Prepare the
cleanup off-tree during the source hold, then fix it after SpecTcl/Jim triage;
no new lint allowances. A selected `--no-deps` run does not clear the full gate.

Actual mechanical shadow patch is ready at
`/tmp/vm-runtime-lint-shadow/mechanical.patch`, SHA-256
`6d93623097aff7c10061ca9138c4eff0be15a271f30fd1a6724051fd567762c9`.
It contains **149 edit groups across 44 files**: identity conversions, docs,
unit patterns/semicolons, method closures, unnecessary borrows, declaration
placement and small equivalent combinators. The receipt beside it records
current base/shadow hashes and every edit. All 44 shadows parsed/formatted with
rustfmt; `git apply --check` passes and base drift is zero at 15:22Z. Root reports its composed VM47 batch applied with the ASCII3 backend; the
first structured check is active. There is no completed current typecheck,
Clippy count or behavioural result from this preparation. TLS retirement `try_with` guards are untouched. Refusal-sensitive
and structural helpers remain in the separate plan for review after acceptance.

Structural preparation is now an actual eight-file shadow draft at
`/tmp/vm-runtime-structural-shadow/structural.patch` (delta after the mechanical
patch); the combined patch is `combined.patch`, SHA-256
`1498e4a51ea0652bb7d4af7754e82e1a79ac29d6b3403d27a2e8ee556051b50f`.
Static review is recorded in `static-review.json`; it corrected helper return
wrappers and reused the existing borrowed scope directly. It names repeated
callback/metadata types, extracts return-error-field and native
list-window helpers, gives bytecode entry an explicit Pending/Entered state,
uses `ControlFlow` for the existing trampoline continuation, and extracts stale
command replay plus wrapped encoder line-length decoding. The receipt records exact canonical/post-mechanical hashes,
review obligations and guard-text counts. Eight shadows parse/format; the
combined patch applies in check-only mode across 47 paths with zero canonical
base drift. Root reports the combined VM47 patch applied; no completed current check or
behavioural run is claimed. These drafts supply no lint-clearance or semantic
admission. The canonical Rust/test freeze is observed, including both D4 files;
latest D4 source write was 15:54:30.558808839Z. No VM cargo process has started.

Required `pack_source_e2e` blocker triage remains read-only. CCG's full-target
receipt reports 3/6; the three expected constants remain absent. The public
helper `/tmp/vm-runtime-derived-probe.rs` was incorporated in CCG's single
combined diagnostic, whose completed output is `/tmp/ccg-n17_public_axes.log`.
No VM Cargo process or canonical Rust/test edits were used for this triage.

CCG's combined public diagnostic now reports both derived Evaluate transports
succeed (42/xy), while their eight compiler rows retain `::p` and known version1
is Overdefined with no alternate version or route entry. Missing publication,
SSA-version mismatch and derived host transport are ruled out by that diagnostic;
compiler owns the remaining pre-route issue. The explicit ASCII-guarded strlen
hook instead reports `native compiler preflight provider required`. Static trace
at `/tmp/vm-runtime-native-hook-provider-triage.json` finds an already installed
compiler and authentic VM entry capture, with private `::tcl::string::is` hook
knowledge blocked by its absent native compilation descriptor. CCG owns the
narrow typed descriptor draft off-tree; no VM installation fix is indicated.
Pinned C8.6.18/9.0.4/9.1.0 source review confirms the exact non-strict ASCII path
compiles one value word then STR_CLASS with no local allocation. Review and
unexecuted oracle draft: `/tmp/vm-runtime-string-is-ascii-c-review.json` and
`/tmp/vm-runtime-string-is-ascii-oracle.tcl`. R1/C6 source/test hold remains active.
These are diagnostic/static findings, not section C/D/F acceptance or lint passes.

Finite native C ASCII controls were executed under root's oracle-only
concurrency authorization. Completed evidence is archived in
`../handoff/reviews/merge-inventory-20261019/vm-runtime/runs/native-ascii-oracle-20261022T171543Z/`.
Pinned C8.6.18/9.0.4/9.1.0 each pass 20 fixed semantic assertions; their
non-strict/strict procedures each contain one `strclass ascii`, while dynamic
class and failindex controls each contain one `invokeReplace` and no strclass.
C8.5.19 semantic-only controls pass 20. C8.4.20 passes the first 16 controls,
then the extra embedded-NUL plain assertion returns 0 versus expected1 (exit1);
that expectation and failure are preserved, and the last three controls do not
run. Receipts include exact argv, script/source/wrapper/executable/libtcl hashes,
stdout/stderr/exit and stable executable/wrapper rechecks. Each log is under5KB.
These are finite original-C observations, not a global recipe/VM proof.

CCG-requested backend draft is prepared ONLY in `/tmp/vm-runtime-string-is-ascii-shadow/`:
`combined-runtime-vm-ascii.patch`, SHA-256
`99c4090107130e629e490ecb76b91831f2393a7442b166f31cb533c57c05e3af`.
It covers runtime scalar artifact execution, the VM's marked ASCII STR_CLASS
worker and explicit enum rejection by the existing length helper. It uses the
original checked Unicode getter and actual C Boolean execution constant;
unmarked legacy operations stay as before and unmodeled marked classes refuse.
All three canonical before hashes remain unchanged; rustfmt parses and application
check passes, but no typecheck/build/runtime/Clippy acceptance was run. The draft
depends on CCG's typed ASCII-only descriptor/enum and root's later explicit release.
No VM Cargo process has started; after8b/Jim/TLS remains first execution priority.

All 17 prior `jim_document` failures now have an explicit source/log
classification in `../handoff/reviews/merge-inventory-20261019/vm-runtime/jim-document-source-triage-20261022.json`.
Three procedure-shape cases directly encounter the missing typed descriptor;
four class-list cases use separate retained source-purpose/original-list
premises; five member/receiver joins, three disabled-availability diagnostics,
one distinct two-word unknown command and one compound program remain separate
obligations. No current Rust test or pass-count is inferred from this triage.
Pinned Jim0.84 passes 25 finite semantic controls (proc3/4/static copy/reference,
class dictionaries/inheritance/methods and invalid shapes), archived under
`runs/vm-runtime-jim-document-oracle-20261022T172713Z/`. This preserves the original
semantic expectations without granting analyser admission.

A minimal Jim typed descriptor draft is ready ONLY in
`/tmp/vm-runtime-jim-descriptor-shadow/trusted-jim-proc.patch`, final SHA-256
`346d63ea42ce2886b30766c91145b79061539bd28ab362b5a84c81070c9cb32b`.
Compiler statically approved its boundary and direct helper negatives; root
applied it in the R3 source window. Ordinary pack restrictions and no-fallback
consumer semantics remain. The current Jim document execution is pending.

The scratch runner is `/tmp/vm-runtime-preflight/seq-safe.sh`: one `cargo test`
build/run per target, draining filtered logs capped at 20 MB, recording exit and
warning counts, no deletion of fresh shared executables. Source
`/tmp/tcl-lsp-naming-env.sh` first. The old runner below is historical and has
hardcoded paths plus unsafe fresh-executable cleanup; do not execute it unchanged.

Narrow additional ownership agreed with lsp: extend
`rust/tcl-lsp-server/tests/e2e/spec_packs.rs` bundled EDA smoke to six vendor
environments. Server lib and general section D lifecycle seams stay with lsp.

**After8 (review C.2 Jim/EDA targets).**
- Finished: `jim_document` **9 passed / 17 failed / 26**. The panic summaries are in
  `…/vm-runtime/runs/after8.jim_document.failures.txt`. No pre-change baseline
  exists. All 17 have now been classified by source/log family in the triage
  receipt above; the three direct proc-shape failures have a narrow descriptor
  draft, while class-source and diagnostic-provenance causes remain separate.
  Guarded current rerun is still pending; no general VM parity repair is shown.
- Not run: `jim_own_surface`, `jim_surface_roster`, `jim_generation_zero`,
  `jim_seed_after_publish`, `eda_loadables`, `eda_environment_packs`.

**Exact next command after root releases the VM slot.** Active logs stay under `/tmp`;
copy completed receipts into the evidence tree only after the target finishes:

```
source /tmp/tcl-lsp-naming-env.sh
TMO=1800 bash /tmp/vm-runtime-preflight/seq-safe.sh after8b-resumed tcl-spectcl:jim_own_surface tcl-spectcl:jim_surface_roster \
  tcl-spectcl:jim_generation_zero tcl-spectcl:jim_seed_after_publish tcl-spectcl:eda_loadables \
  tcl-spectcl:eda_environment_packs
```

Then:
1. Triage the `jim_document` failures.
2. `FILTER="--exact every_shipped_tclspec_loads_installs_and_analyses_against_corpus" THREADS=1 seq.sh after9 tcl-spectcl:spec_corpus`
3. `FILTER="--skip every_shape" seq.sh after9 tcl-spectcl:codegen_stamps`
4. The runtime with libtommath:
   `cd runtime/rust && TCL_TOMMATH_DIR=../../tmp/tcl9.0.4/libtommath cargo test --locked --features engine`
5. The runtime without libtommath:
   `unshare -m sh -c 'mount --bind <emptydir> tmp/tcl9.0.4/libtommath && cd runtime/rust && cargo test --locked'`
6. `cargo test -p tcl-vm --lib`, and compare it with
   `…/vm-runtime/sol48-baseline/before.tcl-vm---lib.status`.
7. `cargo test -p tcl-engine-tclvm --lib`
8. `seq.sh after10 tcl-vm:embed_api_e2e`
9. Clippy `-D warnings` on my crates.
10. Finish `vm-runtime-NOTES.md`, which has `RESULTS`/`COMPILER`/`REMAINING` placeholders.

Re-check the thread-exit fix with:
`cargo test --locked -p tcl-vm --test thread_exit_retirement -- --test-threads 1` (2/2).

**Files touched this session** (all uncommitted edits are now in the working tree for the
coordinator's commit):
- `rust/tcl-spectcl/src/loader/eval.rs`: `static_row`, the fast-path brace fold.
- `rust/tcl-spectcl/tests/`:
  - `codegen_stamps.rs`: `PlainCounting` forwarding.
  - `upf_hooks.rs`: new.
  - `spec_corpus.rs` and `spec_corpus_baseline.txt`: oo-class lines removed.
  - `golden/*.snap`: regenerated.
- `rust/tcl-vm/src/value/native_retirement.rs`, `value/native_jim_pool.rs`, `value.rs`:
  the thread-exit `try_with` changes.
- `rust/tcl-vm/tests/thread_exit_retirement.rs`: new.
- `rust/tcl-vm/src/cmd_package.rs`: `version_conflict` takes the protocol release.
- `rust/tcl-engine-tclvm/src/lib.rs`: a doc comment.
- `runtime/rust/src/{value_ops.rs,interp/native_variable_names.rs,interp/native_body_artifact.rs}`:
  redundant `#[path]` attributes removed. NOT yet compile-checked; that is runtime step 4.
- `docs/design/spec-dsl-examples/{oo-class,upvar,lsort,switch,foreach,return,string}.tclspec`
  and `README.md`.
- `rust/tcl-spec-studio/tests/{spectcl_ports,pack_store}.rs`.

**Evidence** is under `docs/work/name-handling/handoff/reviews/merge-inventory-20261019/vm-runtime/`:
- `runs/*.summary`, `*.status` and `after8.jim_document.failures.txt`;
- `sol48-baseline/` (per-test before statuses);
- `gold/` (classification scripts and table);
- `zz_vmrt_probe.rs.txt` (the golden-render probe);
- `seq.sh`.

## In progress

- Waiting for compiler-names, compiler-codegen and compiler C6 build slots in that
  order; root owns sequencing. The initial waiting flock was cancelled before any
  VM-runtime cargo process started. Active logs will remain under `/tmp` and only
  completed receipts will be copied into evidence. Six-vendor EDA smoke is prepared;
  planning the external Clippy inventory off-tree without dependency source edits.
  The remaining Jim/EDA targets are queued. Earlier `jim_document` is 9/17/26;
  its failures include signature/static/body analysis and disabled-command diagnostics
  as well as class facts. This target does not execute VM code or external oracles.

## Outstanding (in priority order)

1. **`spec_corpus` `every_shipped_tclspec_loads_installs_and_analyses_against_corpus`.**
   - Rerun it alone, single-threaded. Its last run hit the 600 s watchdog under load.
   - It must confirm the `spec_corpus_baseline.txt` edit, which removed the 9 obsolete
     oo-class lines and their header paragraphs.
2. **`codegen_stamps`: collect the failure messages.**
   - Rerun with `--skip every_shape`. The last run was 5 ok / 23 failed / 1 still running
     at the 1500 s cap; the before figure was 1/28.
   - Wait for compiler-codegen's `install.rs` native_compilation change, then re-classify
     which failures are VM-side and which are compiler-side.
3. **runtime/rust.**
   - `cargo test --locked --features engine` with libtommath: the lib `string` tests,
     `integer_argument_reads`, `literal_index_encoding`, then the rest.
   - The no-libtommath configuration: hide the tree only in a private mount namespace
     (`unshare -m` + bind-mount an empty dir), never by moving the shared tree.
   - These also compile-check this session's redundant `#[path]` removals.
4. **tcl-vm lib** (Sol-48 baseline 1107/1387, per-test log retained),
   **tcl-engine-tclvm lib** and **embed_api_e2e**.
   - These check the Sol-48 commands-budget change and the `cmd_package`
     `version_conflict(release)` change.
   - Report every test that passed before and fails now.
5. **Clippy `-D warnings`** on changed lines in my crates (whole-crate where cheap), and
   `cargo fmt` (edition 2021 for runtime/rust).
6. **The vm-runtime handoff.**
   - `docs/work/name-handling/handoff/reviews/merge-inventory-20261019/vm-runtime-NOTES.md`
     is drafted; fill in the results.
   - Write `vm-runtime.patch`.

## Blocked (on whom, and what is needed)

- **`pack_source_e2e` 3/6** (fold DSL 1, reference body, derived body), on compiler-codegen:
  - `optimise_raw` never invokes installed pack const_fold hooks. The host error log and
    crash log are empty, and there is no O129.
  - The TclVmEngine sandbox refuses `string cat` / `string is` bodies with
    `NativePreflightRequired` on the entry-aware route. `string length`, `lindex` and
    `expr` work.
  - Both are routed by the coordinator.
- **`spec_corpus` hostile-pack test** (hooks attempted 0/0): same root cause as the
  `optimise_raw` hook gap, on compiler-codegen.
- **`spec_corpus` `shared_unit_matches_legacy_for_representative_profiles`**: on the
  compiler agent. tcl8.6 `if {$value}` gives I230 from the legacy path, but the shared
  path gives nothing.
- **`codegen_stamps` remaining failures**, on compiler-codegen:
  - `BytecodeCompileService::default().compile()` refuses every source;
  - a whole-module compile emits no procedures or provenance and no pack
    specialisation/claims;
  - native_compilation is not derived for pack rows (`install.rs`, in progress there).
  - `every_shape_of_inlined_body…` takes more than 25 min, because every compile
    re-assembles `ContextRegistry` (compiler lowering cost).
- **N17** (codegen `$a:b` variable grammar → wrong registration): compiler-codegen.

## Done since the last update (with evidence: test counts, commit or snapshot)

- **Thread-exit abort fixed** (priority, from the lsp agent).
  - The drop paths in `tcl-vm` `value/native_retirement.rs`, `value/native_jim_pool.rs` and
    `value.rs` (`DoubleFormatContext`) now use `LocalKey::try_with`, and leak instead of
    panicking once a thread-local is gone.
  - New `tcl-vm` test `thread_exit_retirement` passes 2/2.
  - Negative control: with only `release` reverted to `PENDING.with`, the test binary
    aborts with the reported AccessError (exit 134).

The state is in WIP snapshot `56893780b9`, plus the working tree since.

- **`eval_loader` 28/28** (was 27/1/28).
  - Static fast path folds braced backslash-newline (`loader/eval.rs` `static_row`).
  - Checked against tclsh 8.4 to 9.1, which all fold.
- **`golden_packs` 3/3** (was 1/2/3).
  - The goldens were regenerated twice under the classification rule: 1367 digest-only
    commands (proven by hash), 148 fold commands, and the accepted foreach line plus the
    oo-class port.
  - After the port pass, only the intended fields changed: foreach, lsort, return,
    string, switch, upvar.
- **`pack_is_real_tcl` 1/1.**
- **`upf_hooks` 2/2 (new)**: all 17 UPF hook slots run on the production TclVM host with
  exact expected answers.
  - 16 option-arity hooks: optional `TRUE|FALSE` or name, consume 1/0.
  - The `define_power_model` resolver marks the trailing Body, and abstains for one word.
- **tcl-spec-studio `spectcl_ports` 12/12** (was 10/12) and **`pack_store` 7/7** (was 6/7).
  - The oo-class port uses the current vocabulary.
  - The upvar port uses `-level-word Upvar`: tclsh 8.4/8.5 give "wrong # args" for
    `upvar 1 b`, while 8.6+ alias it.
  - Ports aligned with their shipped specs: lsort (and moved to vocabulary 2.1), switch,
    foreach, return, string `is` refine.
  - Non-authorable fields are documented, never loosened.
- **`codegen_stamps` harness**: `PlainCounting` now forwards every compile entry. This is
  +4 passes, with no regressions.
- **Other code changes**:
  - `cmd_package` `version_conflict` takes the protocol release.
  - Redundant runtime `#[path]` attributes removed (owner gate).
  - The engine wall-clock test comment is corrected.
