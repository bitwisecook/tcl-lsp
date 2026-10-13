# Current name conformance measurements

The actual native runtime has incomplete naming conformance. Its frozen
executable uses the selected native core at allocation time and real matching
upstream libraries. Completed assertion results, host refusals and unexecuted
suite regions remain separate evidence.

## Shared namespace-variable query consumers

The [purpose-selected owner and Runtime capture](variable-namespace-query-bases-20261013/README.md)
records16 shared-owner/C5/Jim conformance controls and48 actual Runtime query
comparisons passing. The Runtime capture independently covers every compiled
input and preserves initial test-build failure and corrected build separately.

The [actual VM capture](vm-namespace-query-20261013/README.md) executes13 selectors:
6 pass and7 fail. Accepted scope is12 C84 ASCII namespace-operation rows,
two retained/retired namespace controls, and24 C84/C85/Jim variable-query rows.
Four C85+ namespace tests wrongly require global errorCode publication after
direct object entry; their full results are not accepted. Three C86+ query
fixtures refuse compiler preflight before producing a result. None establishes
Core, F5, ordinary scalar/element behavior or complete naming conformance.

## Actual C8.6 constructor and startup

The [constructor controls](native-c86-mathfunc-bootstrap/README.md) compare actual
C API and Native process results for the original 29-name constructor roster,
procedure birth shapes, replacement currency and matching real initialisation.
All ten fresh-process controls complete; Native real init reaches its marker.
These startup controls do not establish general naming conformance.

[Eight unchanged C8.6 naming suites](runtime-c86-real-suites/README.md) use real
matching init/Tcltest and upstream single-process constraints. C completes with
1,182 passes,85 skips,0 failures. Native proc/rename complete with40 passes,
17 skips,0 failures; five files refuse and ooNext2 exits with a script error.
Those incomplete files retain34 assertion failures; no full Native total exists.
The earlier upstream all.tcl capture fails before any suite on the missing
clock worker `ParseFormatArgs`. The [fresh worker capture](clock-parse-format-args-runtime/README.md) advances past that worker and stops before suites on `getenv`. The [byte-host getter capture](runtime-c86-real-suites/getenv-runtime/README.md) advances past `getenv` and refuses native bytes-to-Unicode conversion before suites. Discrete questions retain exact files,
streams, skips, failure IDs, original orchestration and two diagnostic harness
attempts; only the final upstream-consistent attempt supplies conformance counts.

## Clock orchestration worker

The [eleven original ParseFormatArgs inputs](clock-parse-format-args/README.md)
were measured in all five real C releases and pinned Jim. C8.5/C8.6 each return
four successes and seven worker errors; other providers lack this private
command. Error wording/codes differ between the two responding releases, and
both accept the retained unsigned-boundary clock value. This answers worker
behaviour. The [selected runtime implementation](clock-parse-format-args-runtime/README.md) now matches all eleven C8.6 answers and passes three object/bootstrap controls; full shared-worker pool lifetime and public clock conformance remain open. The getter implementation passes seven runtime controls, one real Unix host primitive and one registry unit; env synchronisation differences remain measured and open.

The package compilation failure at the ordered comparison fixture is [repaired and measured](registry-ordered-probe-api/README.md): one existing control passes,26 filtered tests remain unmeasured.

## Actual VM C8.6 constructor

The [shared-roster VM regression](vm-c86-mathfunc-bootstrap/README.md) passes
through the actual Native constructor and real compiler adapter. It measures
procedure birth headers and retained-body cache invalidation. Real VM init
refuses `NativePreflightRequired`; no startup or suite result follows. The
legacy C9.1 control also refuses before VM execution; 26 lib-test compilation
errors outside changed owners have an unmeasured historical baseline.

## Actual native runtime: Tcl 8.5.19

[Current six-suite report](runtime-current-tcl8.5.19/RESULTS.md),
[records](runtime-current-tcl8.5.19/records.json) and
[provenance](runtime-current-tcl8.5.19/provenance.json) identify the immutable
native binary, real 8.5.19 init/Tcltest, input hashes and exact commands.

| File | C passed / skipped / failed | Runtime passed / skipped / failed |
| --- | --- | --- |
| namespace | 280 / 1 / 0 | Host refusal at `namespace-8.1`; later tests unmeasured |
| var | 69 / 17 / 0 | 66 / 17 / 3 |
| upvar | 52 / 7 / 0 | Host refusal at `upvar-1.1`; later tests unmeasured |
| rename | 11 / 8 / 0 | 11 / 8 / 0 |
| set | 63 / 1 / 0 | 63 / 1 / 0 |
| proc | 25 / 8 / 0 | 25 / 8 / 0 |

C completes all six files with 500 passes, 42 skips and 0 failures. The four
completed runtime files record 165 passes, 34 skips and 3 assertion failures.
Those failures are `var-7.16`, `var-7.17` (zero-argument `variable`) and
`var-14.2` (`array names -glob`); all three execute and pass in C. They do not
establish three independent defects. Namespace requires original local-alias
table receipt policy and upvar requires original simple-variable-cell policy.
Both terminate through explicit host refusals without complete totals.

The [namespace/ensemble lifecycle report](runtime-lifecycle-fix/RESULTS.md)
retains seven passing exact native regressions and matching C 8.5–9.1 ensemble
controls. The exact upstream `namespace-56.2` body now passes with real 8.5
init/Tcltest. A cached namespace reference after 8.5 initialization still differs
from C. The retained lifecycle capture stops before that test on 8.4, 8.6, 9.0 and
9.1 through explicit missing native-policy refusals; no whole-file suite result
is claimed for those subjects.

The immutable current runtime binary is
`/tmp/name-runtime-native-ensemble-fixed`, SHA-256
`eaf091d49143d47282b825ffff53012b799b72acaec1780b6326207728bb180f`.
Its [build receipt](runtime-current-tcl8.5.19/runtime-build-receipt.json) and
[archived build provenance](runtime-lifecycle-fix/native-ensemble-fix/build-provenance/README.md) identify
commit `c49ef6fd5d147edd0cf473070dfaac2834c20dd7` plus captured uncommitted
changes. [Discrete questions](runtime-lifecycle-fix/native-ensemble-fix/README.md)
pair exact inputs or frozen consumer-test definitions with their scoped results.
The broad source snapshot was recorded after compilation while agents shared
source; it does not prove stable compiler inputs. Later source edits do not
inherit these executable outcomes.

## Real Tcl 8.4 naming suites

The [six-suite original 8.4 capture](runtime-c84-real-suites/20261012T151253Z/README.md)
uses the frozen declared-cell runner with real matching init/Tcltest. C has
579 passes, eight knownBug skips and no failures. Four runtime files complete
with 270 passes, three skips and ten namespace assertion failures; var/proc/rename
pass. Upvar stops at its first case on an original simple-cell refusal. Interp
records 13 assertion failures before a nested-alias source-interpreter refusal
at interp-11.4, with no complete runtime summary for either aborted file.

These measurements certify their captured executable/input hashes, not later
edits or another release. Per-suite questions, exact upstream source, raw streams
and constraints are linked from the capture.

## Separate frozen subjects

The remaining report retains historical measurements of commit
`b7ad30f1fcc1911f8e8744485d809f5348b9b850` and its recorded drivers. Its
focused passes, startup observations, suite failures and totals belong only to
that frozen subject. In particular, those runtime driver results used Logical
construction and do not establish the current actual native core's behavior.
Their existing input/raw/provenance captures remain useful oracle and
reproduction evidence, without current VM or runtime conformance claims.

# Frozen b7ad VM/runtime measurements

The existing focused regressions pass, but the original upstream naming suites
expose substantial remaining gaps. Neither engine has complete naming
conformance. Successful shared lookup vectors do not establish namespace
lifetime, variable-cell, alias, introspection or TclOO conformance.

## Measurement basis

Subject: `work` commit `b7ad30f1fcc1911f8e8744485d809f5348b9b850`.
No VM/runtime/compiler semantics or upstream expected results were changed.
Builds use Rust 1.99.0 on native Linux, separate Cargo targets, and explicit
Tcl 9.0.4 libtommath for the standalone runtime. This measures the native
bytecode VM and the native build of `runtime/rust`, not a compiled WASM guest.

Both engines initialise from real upstream `init.tcl` and discover the real
`tcltest.tcl` through the matching library. Tcl 9.0.4 reports Tcltest 2.5.11.
[VM startup](focused-regressions/name-vm-startup.txt) and
[runtime startup](focused-regressions/name-runtime-real-library.txt) retain
library/loader evidence. Source, library, executable and input hashes are in
each capture's provenance and per-test-file records.

The primary comparison runs 21 original Tcl 9.0.4 `.test` files on C Tcl, VM
and standalone runtime. Six core files additionally run on C/runtime at
8.4.20, 8.5.19, 8.6.18 and 9.1.0. Seven exact upstream IDs run independently
on all three Tcl 9.0 engines to check whether whole-file failures reproduce.
Shared naming-vector conformance also runs against all five exact C references.

The [capture tool](../run_suites.py) runs the original test files, with bounded
processes and temporary work directories, and preserves stdout/stderr, status,
failed IDs, last started ID, wrapper inputs and provenance. A successful capture
exit means records were saved. Tcltest failures can leave a process exit of zero;
an aborted file has no complete totals. For VM, `test_sha256` identifies the executed original file; the retained
`input.tcl` configuration template is not passed to that driver. Snapshot source
is retained as text so
formatters cannot silently change its recorded hash.

## Focused regression results

| Selection | Result | Evidence |
| --- | --- | --- |
| VM naming/OO/alias/interpreter/trace integration selections | 333 passed in 16 binaries | [log](focused-regressions/name-vm-integration.txt) |
| VM unit suite | 109 passed | [log](focused-regressions/name-vm-all-units.txt) |
| Runtime name/namespace/OO/alias/trace/frame unit and integration selections | 350 distinct tests passed | [units](focused-regressions/name-runtime-units.txt), [additional selections](focused-regressions/name-runtime-frames-lookup.txt), [integrations](focused-regressions/name-runtime-integrations.txt) |
| Shared command, variable and namespace-op conformance | 7 Rust tests passed, real C matrix 8.4–9.1 available | [log](focused-regressions/name-shared-conformance.txt) |

Runtime counts deduplicate overlapping module selections. Its integration
suite includes live upstream trace cases through real Tcltest startup. These
passes demonstrate the selected regression coverage, not whole-suite parity.

## Whole Tcl 9.0.4 files

Completed cells are **passed / skipped / failed**. An aborted or timed-out
cell reports the stop point and observed failing IDs before termination, not
invented totals. See [machine-readable records](naming-suites-tcl9.0.4/records.json).

| File | C Tcl | VM | Runtime |
| --- | --- | --- | --- |
| namespace | 311 / 3 / 0 | process-error at `namespace-56.2`; 42 failures recorded | process-error at `namespace-56.2`; 19 failures recorded |
| namespace-old | 126 / 0 / 0 | process-error at `namespace-old-6.20`; 8 failures recorded | 124 / 0 / 2 |
| var | 200 / 21 / 0 | 163 / 21 / 37 | 186 / 21 / 14 |
| set | 63 / 1 / 0 | 63 / 1 / 0 | 63 / 1 / 0 |
| set-old | 153 / 0 / 0 | 94 / 0 / 59 | 93 / 0 / 60 |
| upvar | 62 / 8 / 0 | 45 / 8 / 17 | 56 / 8 / 6 |
| uplevel | 57 / 0 / 0 | 55 / 0 / 2 | 41 / 0 / 16 |
| proc | 29 / 9 / 0 | 29 / 9 / 0 | 29 / 9 / 0 |
| proc-old | 74 / 0 / 0 | 64 / 0 / 10 | 71 / 0 / 3 |
| interp | 339 / 14 / 2 | process-error at `interp-2.13`; 6 failures recorded | timeout at `interp-34.2`; 95 failures recorded |
| rename | 11 / 8 / 0 | process-error at `rename-6.1`; 0 failures recorded | 11 / 8 / 0 |
| trace | 273 / 17 / 0 | process-error at `trace-34.1`; 11 failures recorded | 243 / 17 / 30 |
| info | 277 / 5 / 5 | 112 / 6 / 169 | 224 / 14 / 49 |
| cmdInfo | 0 / 12 / 0 | 0 / 12 / 0 | 0 / 12 / 0 |
| resolver | 0 / 10 / 0 | 0 / 10 / 0 | 0 / 10 / 0 |
| unknown | 7 / 0 / 0 | 7 / 0 / 0 | 7 / 0 / 0 |
| oo | 372 / 16 / 0 | process-error at `oo-6.19`; 51 failures recorded | 357 / 16 / 15 |
| ooNext2 | 57 / 5 / 0 | 18 / 5 / 39 | process-error at `next-tailcall-simple-1`; 3 failures recorded |
| ooProp | 55 / 0 / 0 | 55 / 0 / 0 | 55 / 0 / 0 |
| ooUtil | 33 / 0 / 0 | 2 / 0 / 31 | process-error at `ooUtil-1.7`; 5 failures recorded |
| apply | 38 / 4 / 0 | 30 / 4 / 8 | 32 / 4 / 6 |

Across completed files, VM records 737 passes and 372 failed assertions;
runtime records 1,592 passes and 201 failed assertions. These totals exclude
unfinished files. Including failures printed before aborts/timeouts, there are
485 VM and 323 runtime failed IDs whose corresponding C test executed and
passed. These are observations, not counts of independent bugs: they include
cascades, error-format differences and capability/internal-representation cases.
Do not derive a conformance percentage from these incomplete executions.

C's seven residual failures are harness state/topology effects: five
`info-22.*` tests assume a fixed absolute frame depth/source launch, and
`interp-38.4/.5` expect frame debugging off after the capture enables it.
They are excluded from C-passing failure counts. Both `cmdInfo` and `resolver`
execute zero tests on all engines because their C test-command constraints
are absent. An all-skipped file is unmeasured, not a pass.

The broad `info`, `interp` and `set-old` files also test capabilities beyond
name lookup, including zlib stream handles, command limits, command counting,
array searches and C bytecode metadata. Keep these separate from ordinary
name/cell semantic defects. The unmodified files expose dependencies that
prevent complete naming coverage; they are not all resolver defects.

## Release-specific startup and core suite coverage

Each row covers `namespace`, `var`, `set`, `upvar`, `proc` and `rename` with
its matching real upstream library and suite. All C controls in these rows pass
their runnable tests. Full per-release records remain linked below.

| Release | Runtime outcome | Evidence |
| --- | --- | --- |
| 8.4.20 | Real init succeeds far enough to enter the capture, but loading real Tcltest fails on missing `string bytelength`; all six files are blocked before test summaries. | [records](naming-matrix-tcl8.4.20/records.json) |
| 8.5.19 | `set` and `proc` pass; namespace has 24 failures, var 10, upvar 6, rename 2. | [records](naming-matrix-tcl8.5.19/records.json) |
| 8.6.18 | `set`, `proc`, rename pass; var has 11 failures, upvar 5; namespace terminates at recursive deletion. | [records](naming-matrix-tcl8.6.18/records.json) |
| 9.0.4 | Full 21-file comparison above. | [records](naming-suites-tcl9.0.4/records.json) |
| 9.1.0 | Real library initialisation fails: `invalid command name "tcl::InitAutoPath"`; all six files are blocked before Tcltest startup. | [records](naming-matrix-tcl9.1.0/records.json) |

The Tcl 8.4 capture omits the repository's backend overlay because that script
uses Tcl 8.5+ syntax; upstream constraints still apply. The VM's existing
`run_test` driver is fixed to Tcl 9.0, so it cannot truthfully run this whole-file
matrix at other versions. Its cross-version regression/vector tests do not
replace a version-selectable upstream-suite driver.

Latest Jim's equivalent suite and BIG-IP/F5 measurements were not executed.
Their discovery, runtime/profile coverage and appliance answers remain separate
obligations in the completion plan.

## Independently reproduced failures

[Exact-ID captures](isolated/RESULTS.md) run the original files with one upstream
ID selected. All seven C selections pass; filter-excluded tests are not evidence
for other IDs.

- `namespace-56.2`: both engines terminate after mutually entangled namespace
  deletion traces. Runtime loses `set`; VM loses `list`. The trace scripts
  contain an empty namespace operand. This reproduces independently of prior
  selected test bodies; preserve global/root and retained namespace semantics
  during re-entrant teardown.
- `rename-6.1`, `trace-34.1` and `oo-6.19`: VM independently
  terminates with `expected boolean value but got ""`. The first exercises a
  renamed builtin and compiled code invalidation; the others exercise trace,
  and object-command changes. The common symptom does not
  prove one common cause. Retain each witness and investigate binding/guard
  state and Tcltest's command mutations.
- `interp-2.13`: the VM capture independently stops during file-level child
  cleanup with `could not find interpreter "a x2"`. Investigate interpreter
  path/name identity and cleanup; the last started ID is not proof that its
  selected test body raised this error.
- `trace-34.1`: runtime returns an extra execution-trace callback after
  remove/re-add. Trace membership and invocation-window semantics differ from C.
- `ooUtil-1.7` and `next-tailcall-simple-1`: runtime terminates restoring `puts`
  with `can't rename to "::puts": command already exists`. TclOO library tests
  intercept output through rename/import; their restore path exposes command
  identity/lifecycle behaviour.

Runtime `oo-6.19` and `interp-2.13` pass both in isolation and in whole-file
execution. Their containing files have other failures or unfinished coverage.
The VM passes the selected tailcall case independently. Preserve sequence-level
runs and reduce ordered state changes; do not equate all observed failures with
independent API defects. Runtime interp's 90-second whole-file timeout occurs at
`interp-34.2`, involving a million-iteration loop under a child command limit.
The timeout alone does not establish whether the limit is missing or execution
is too slow.

## Semantic gap groups

| Priority / owner area | Concrete measured gaps | Witnesses / relevant existing trackers |
| --- | --- | --- |
| Namespace and command lifecycle | Re-entrant deletion loses builtins; deleted-but-retained namespace path visibility differs; rename/import/trace restoration and compiled binding invalidation stop suites. | namespace-56.2, namespace-7.9, namespace-51.13, rename-6.1, ooUtil-1.7; #1755, #1778, closed lifecycle regressions #1751–#1753/#1763/#1764 |
| Variable cells and links | Writes through links into deleted namespaces/arrays can recreate cells where C errors; self-links/cycles and existing/traced local destinations are accepted incorrectly. | var-1.15–1.18, upvar-6.4, upvar-8.4–8.8 |
| Frame and qualification identity | VM `global` loses qualified/empty local tails; runtime namespace-eval caller frames fail upvar reads; link existence and local introspection disagree. | var-6.1–6.3, var-3.10, namespace-old-7.9/.11, upvar-3.1/.2; #1796, #2311, #2259, #2410/#2412/#2414 |
| Lookup visibility and validation | `info commands` misses namespace-path visibility; forget validation, empty namespace creation, child unknown context and arity handling differ. | namespace-51.14, namespace-10.6/.7/.9, namespace-14.13, namespace-40.1, namespace-52.11; #2300, #2292 |
| Arrays and observable storage | Array search operations absent; alias-to-element array mutations and defaults disagree; trace recovery/deleted-cell lifecycle differ. | set-old-8–11, var-24.*, trace-1.8/4.4/4.5/6.2/6.3 |
| TclOO and namespace libraries | Definition-body keyword versus same-named class/mixin command collisions; forwarded `my` mutation; call-chain/next/nextto and `oo::class` library utilities need broader coverage. | oo-6.19, oo-18.10, ooNext2, ooUtil; #1778 and existing OO token/method-context regressions |
| Errors and presentation | Several results compute the same basic effect but carry wrong operation names, invocation prefixes, namespace context, errorCode or introspection order. | var-25.6/.7, set-old-7.4, upvar-NS-1.*, oo-2.7, var-1.20; #2342/#2343 |
| Reference bootstrap / harness | Runtime 8.4 Tcltest load and 9.1 init blocked; VM whole-suite runner fixed to 9.0; resolver/cmdInfo internal test APIs unmeasured. | Per-release matrix and startup logs |

For example, `var-1.15` should reject writes through an alias into a deleted
namespace; runtime instead returns a successful scalar write and later a
scalar/array error. `namespace-old-7.9` should read a namespace-eval caller's
variable but runtime reports no such variable. `namespace-51.14` tests live
path-based command enumeration even when ordinary dispatch vectors pass.
`var-1.20` reports reversed Unicode local-name enumeration, not a proved Unicode
lookup failure. `set-old-7.4` differs only in whether its error says “no such
variable” or “variable isn't array”; do not describe it as successful mutation.

Detailed IDs, bodies and actual/expected errors are retained per stem in
`naming-suites-tcl9.0.4/<engine>/<stem>/stdout.txt` and `record.json`. The
failure-ID count is useful for selecting regressions, not for assigning all
failures to a single centralisation issue.

## Work required to finish

1. Restore real 8.4 Tcltest and 9.1 init bootstrap. Make VM upstream execution
   release-selectable, while validating exact source/library/interpreter pairs.
2. Reduce and fix the suite-stopping lifecycle/binding cases first. These block
   later naming coverage. Keep both exact-ID and whole-file sequence tests.
3. Consolidate namespace/command policy over identity-preserving state adapters:
   global/root handling, retained namespaces, command tokens, imports versus
   late-bound aliases, trace windows and cache invalidation.
4. Consolidate variable place/frame/link resolution: qualified and empty names,
   namespace-eval frames, dead cell rules, cycle/destination checks, array aliases,
   defaults and trace recovery. Propagate these identities into compiler facts.
5. Finish path/unknown/forget/introspection validation and TclOO definition/
   method/library namespace consumers. Separate effect bugs from diagnostic and
   presentation differences, retaining exact C observations for both.
6. Promote measured upstream IDs into shared deterministic fixtures and each
   affected consumer's tests. Add original-suite coverage so focused green tests
   cannot mask missing surfaces. Keep C-only bytecode probes explicitly scoped.
7. Rerun affected files after fixes, then the complete comparison matrix. Check
   actual executed IDs, skips and stop points. Complete Jim, F5, WASM/codegen and
   static LSP/tooling surfaces before claiming the whole programme finished.

## Retained capture variants

`current-engines-tcl9.0.4` retains the extra-source-frame capture and the rejected
shell-wrapper controls. `reference-controls-tcl9.0.4` retains physical-source
controls without frame-debug setup. They document how launch context changes
measurements; neither supplies the primary parity table. The primary capture
uses stdin configure/source for C/runtime and the original file directly for
VM, with real source libraries. C frame-debug state is recorded explicitly.
No upstream expected value was edited to accommodate any capture.
