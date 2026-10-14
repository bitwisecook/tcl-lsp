# Name-handling oracle experiments

Reproducible programs and measured outcomes for central name handling.
The consumer/API entry point is
[the shared contract](../../docs/design/contracts/name-handling.md).
Existing command/variable/namespace vectors live in
`rust/tcl-syntax/tests/data/`; existing BIG-IP experiments live in
`scripts/dev/bigip-probes/`. Reuse those owners and keep their original evidence.

## Assets

| Path | Meaning |
| --- | --- |
| [cases.tcl](cases.tcl) | Portable C Tcl 8.4-compatible observation suite; executable measurements, no manufactured expected answers |
| [case-catalogue.md](case-catalogue.md) | Exact case IDs, semantic questions and consumer coverage |
| [pattern-sources.md](pattern-sources.md) | Maintained corpus and Tcl Wiki discovery, provenance and held-out validation |
| [patterns/README.md](patterns/README.md) | Maintained wild-pattern catalogue: sources, experiments, per-oracle results and unanswered questions |
| [patterns/corpus-findings.md](patterns/corpus-findings.md) | Pinned corpus sites, reduced observations and bounded recogniser requirements |
| [patterns/wiki-findings.md](patterns/wiki-findings.md) | Actually read Wiki sources, positive/adversarial probes and existing owner mappings |
| [array-mode-validation/README.md](array-mode-validation/README.md) | Discrete C/Jim mode/trace questions, release-specific outcomes and native comparisons |
| [results/native-variable-array-modes/README.md](results/native-variable-array-modes/README.md) | Variable arity and array-name controls, VM/runtime matrices and real 8.5 var.test |
| [results/native-c84-declared-cell/README.md](results/native-c84-declared-cell/README.md) | Existing physical-cell observation and real C 8.4 startup |
| [results/runtime-c84-real-suites/20261012T151253Z/README.md](results/runtime-c84-real-suites/20261012T151253Z/README.md) | Six real 8.4 naming suites, exact questions, inputs, results and refusals |
| [results/RESULTS.md](results/RESULTS.md) | Scoped VM/runtime original-suite comparisons and measured gaps |
| [results/runtime-array-search-var132-current/results.md](results/runtime-array-search-var132-current/results.md) | Original var-13.2 validation dependency: measured iterator ownership repair and exact harness replay |
| [run_suites.py](run_suites.py) | Original-suite capture with matching source-built interpreter, original-input admission and retained provenance |
| [upstream-integrity/README.md](upstream-integrity/README.md) | Immutable original library/test inventories selected through the shared reference manifest |
| [results/shared-name-owner-current-20261013/README.md](results/shared-name-owner-current-20261013/README.md) | Scoped central command/variable/namespace and current Jim name controls |
| [run_jim_suites.py](run_jim_suites.py) | Original upstream Jim naming suites using Jim's actual test wrapper |
| [results/README.md](results/README.md) | Provenance and record format; records distinguish measured, pending and unsupported |
| [Core145 result](results/core-production145-20261014/result.json) and [retention/comparison manifest](results/core-production145-20261014/retention.json) | All196 controls:123 PASS/73 FAIL, source-subset drift0, missing/empty selection0 and no timeouts. All101 Core117 controls retained with three gains/zero regressions;95 additions49 PASS/46 FAIL. Complete202-file capture retained byte-exact; Native minifier editing capability remains an independent obligation |
| [Compiler146 result](results/compiler-production146-20261014/result.json) and [retention/comparison manifest](results/compiler-production146-20261014/retention.json) | All313 controls:248 PASS/65 nonpassing, three harness timeouts, source-subset drift0 and no missing/empty selectors;320 files and three large logs retained losslessly. Comparison preserves264 unchanged originals (13 gains/zero regressions) plus one explicitly oracle-corrected/renamed nested-array control; corrected lineage and prior failures remain distinct from unchanged-original gains |
| [Source owner141 controls](results/private-owner-production141-20261014/result.json) | All23 bounded declaration/consumer controls PASS, source-subset drift0; this validates AST registration rules and supplies no Native or production-semantic acceptance |
| [Owner-resolution CLI143 result](results/owner-cli143-20261014/result.json) and [full log](results/owner-cli143-20261014/owner-resolution.log) | Actual CLI exit1 with207 reported contract failures (196 general owners/contracts and11 Source routes), source/document-subset drift0;484 log lines include warnings, and this scope is separate from the full strict-proof checker |
| [VM132 result](results/vm-production132-20261014/result.json) and [retention manifest](results/vm-production132-20261014/retention.json) | All52 controls:49 PASS/3 FAIL, full source/compiled-input/artifact drift0; all45 prior plus seven additions retained, with two original child regressions and persistent automatic-error corpus failure; complete75-file capture retained byte-exact |
| [oracles/variable-completion-extent-20261014/README.md](oracles/variable-completion-extent-20261014/README.md) | Original C8.4–9.1/Jim variable reference closers; exact source, raw output and retained manifest path mapping |
| [Original Jim child startup probes](results/original-jim-child-startup-20261014/result.json) and [retention manifest](results/original-jim-child-startup-20261014/retention.json) | Three unchanged original Jim scripts with raw output and provider/source hashes, ten files retained exactly: missing-variable omission, global-scope copying and CString extents; no C Tcl, F5 or migrated-runtime equivalence claim |
| [oracles/jim-original-key-reference-observer-20261014/result.json](oracles/jim-original-key-reference-observer-20261014/result.json) | Actual Jim command-key/object reference observation with original observer source, build and raw output; no Native runtime acceptance |
| [f5/REQUEST.md](f5/REQUEST.md) | Appliance-chat questions, execution contexts and return protocol |
| [f5/ANSWER-TEMPLATE.md](f5/ANSWER-TEMPLATE.md) | Answer document template |
| [f5/answers/README.md](f5/answers/README.md) | Destination for returned documents and raw transcripts on `work` |

The portable suite runs in a **fresh disposable interpreter**, not in an
application's interpreter. Its fixed `::__nh_*` names are experiment fixtures.
It does not provide a ready-to-load iRule. The F5 request describes how to
materialise per-context probes with isolated appliance names and dynamic
command construction. Applying a stock interpreter wrapper to TMM would
confound load-time validation with runtime name lookup.

## Oracle identities

| Oracle | Required identity and evidence |
| --- | --- |
| C Tcl | All release lines 8.4, 8.5, 8.6, 9.0, 9.1. Exact pins from `rust/tcl-dialect/data/reference-toolchains.tsv`; validated binary and matching source through `tcl-test-support`. |
| Latest JimTcl | Discover the latest upstream release/revision when running; record upstream ref, commit, reported version, configure/build flags and enabled extensions. A recorded 0.84 baseline is not a claim about latest. |
| F5 Tcl | BIG-IP version/hotfix/build/platform, execution context and exact accepted/running probe. Record TMM iRules, CLI script and iApp separately; appliance tclsh is a control. |

The current C pins are 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1.0. Do not copy
these into another runtime manifest. Use shared test-support discovery,
`TCL_LSP_TCLSH84` through `TCL_LSP_TCLSH91`, and
`TCL_LSP_REQUIRE_TCLSH=1` for witness-based callers. Current shared syntax
vector runners use `available_tclshs()`. Set
`TCL_LSP_REQUIRE_ALL_TCL_ORACLES=1` to require the complete validated matrix,
or select `required_tclshs(TclVersion::ALL)` explicitly. Invalid configured
references fail rather than being skipped.
A locally installed unvalidated interpreter may provide a clearly labelled exploratory measurement,
not a completed reference gate.

## Upstream test suites

C Tcl's upstream tests run through `tcltest` on the matching real C interpreter
are valid oracle evidence. Jim's equivalent upstream suite, run on the
identified Jim build, serves the same role. Reuse relevant upstream name,
namespace, variable, frame, alias and lifecycle cases before writing a bespoke
probe. Use each C release's matching test tree and harness. When using Tcltest,
execute the **original init.tcl and tcltest.tcl for that exact interpreter
version**, with its original package indices and support files. Retain upstream
revision and hashes and verify the actual loaded library/harness, executable
and source pairing. A minimal rewritten startup or another release's harness
does not answer the original test. Do not alter expected values to make a
consumer pass. File existence and interpreter patchlevel checks alone do not
certify original harness bytes.

Keep unchanged upstream execution, diagnostic backend overlays and adapted
reduced probes as distinct channels. Record any overlay and launch/debug
configuration that changes constraints or frame-sensitive expectations.

Record the suite revision, test file and case ID, exact test definition,
support files/packages, constraints, invocation and raw harness transcript.
Keep pass/fail/skip counts and reasons: a constrained or unexecuted case does
not establish its expected behaviour. Preserve actual results and errors when
reducing an upstream test to a shared conformance vector.

The suite runner checks every retained original library/test input against the
manifest-selected immutable upstream Git blob inventory before capture. It
requires the source tree's built interpreter and verifies exact patchlevel,
original library and Tcltest startup. It records observed input hashes
separately from that upstream authority. Backend overlays remain labelled
channels; skipped cases supply no oracle answer.

For C cases, `tcl-test-support::upstream_test_definition` extracts a definition
from a pinned source file using adjacent test markers;
`tclsh_from_source_tree` and `run_script_from_source_tree` keep interpreter and
source/library selection together. Keep required setup/cleanup and harness
dependencies with the extracted case. For Jim, retain equivalent provenance
and use its own harness and expectations where the suites differ.

The harness organises assertions; the real reference interpreter supplies the
oracle result. Running `tcltest` on our VM/runtime is consumer validation
against that evidence. Retain custom probes for missing cases and cross-surface
identity questions. Suite and probe evidence follow the same capture and
promotion rules below.

## Capture protocol

Each experiment answers a named, bounded question. Keep a small question document
with its stable ID, competing outcomes, exact executable inputs and invocation,
provider/version/context matrix, observed results and limits. Link separate raw
stdout/stderr and machine-readable result records; keep setup, reached lookup
and consumer validation distinct. Related cases may share a runner or provider
manifest, but each case must be independently findable and interpretable.

Build provenance is supporting evidence, not the experiment. Record commit and
input/source/binary hashes, and retain only necessary scoped source snapshots
when uncommitted changes affect the executable. Do not capture unrelated working
tree changes, generated proof indexes or tracking documents in a monolithic
patch. Existing broad captures must retain their original provenance and limits;
splitting a patch does not turn it into discrete experiments.

1. Name the question and stable case ID. Record the exact script bytes or
   script path plus SHA-256, environment/context, interpreter/source revision,
   invocation, locale/encoding and relevant build configuration.
2. Capture raw stdout and stderr independently, process exit status, Tcl
   completion code/result, and errorCode/errorInfo/return options when present.
   The portable runner prints `binary scan` hex projections of its current
   ASCII result/errorCode values in Tcl-list records; it catches each case so an
   expected lookup error does not abort
   the whole suite. Its catch currently combines setup and lookup; inspect
   the retained errorInfo and reproduce in isolation before classifying a
   setup failure. Jim/F5 adaptations must record setup and lookup phases
   separately. This process exiting zero is **not** an all-tests pass.
   Before adding Unicode/binary variants, extend capture to distinguish
   Unicode text (explicit UTF-8/code-point representation) from byte arrays;
   `binary scan` alone is not a universally lossless text representation.
3. Keep capability absence, compile rejection, runtime error, successful
   execution, unavailable oracle and infrastructure failure distinct. A rejected
   setup does not prove a lookup result. A C-compatible script may depend on
   namespace/import/interp/package features absent in a Jim build.
4. Repeat a suspicious observation in isolation. Preserve controls and negative
   cases. Avoid unstable object IDs or unordered enumeration in expectations;
   if normalisation is necessary, document precisely which observable data it
   removes and retain the unmodified raw record.
5. Publish an observation/interpretation record linking exact inputs and raw
   outputs. A conclusion is scoped to its tested oracle and context. If two
   contexts disagree, retain separate rows and investigate the relevant axis.

Example local capture (choose a validated reference executable for a gate):

```sh
mkdir -p /tmp/name-oracle-run
/path/to/reference/tclsh experiments/name-handling/cases.tcl \
  > /tmp/name-oracle-run/stdout.txt 2> /tmp/name-oracle-run/stderr.txt
```

Record the shell exit status immediately and move reviewed evidence into a
new provenance-named result directory. Do not overwrite an earlier measurement
when a reference pin changes. Keep compact relevant transcripts rather than
unrelated shell/session diaries or appliance secrets.

## Promotion to tests

Each deterministic expectation must cite an observation ID, input hash, oracle
identity/context and raw result. Map the observed result to release/dialect
columns in existing vector formats where possible. Add a new format only if
the owner cannot express the scenario, and document its consumer contract.

The portable observations are not automatically assertions or C/Jim/F5
agreement. Verify and promote measured outcomes. A consumer's result is compared
to an oracle-derived expectation, never used to generate its own expected value.
The analyser/LSP may abstain under uncertainty; test the justified abstention
rather than requiring execution-only knowledge in a static surface.

Cover grammar/identity owners, affected compiler/optimiser/codegen paths,
VM/WASM, LSP workspace/rename and CLI/MCP where applicable. Share the **same
identity question** across surfaces. Include wrong-target/collision/mutation
negative cases and exact source spans, not just successful ordinary lookup.
Future regressions run from retained fixtures without needing a live BIG-IP;
refresh live evidence when semantics/version support changes. Manual generated
campaigns remain separate from normal deterministic CI tests.

## Evidence retention

Keep executable scripts, controls, machine-readable observations, explanations
of experimental method, provenance and returned appliance answers. These
assets describe experiments and measured facts. They contain no development
phase diary, agent chronology or narrative of abandoned implementations.

## Verified original harness captures

All five release-specific captures below validate matching original init/Tcltest,
suite/support inputs and source-built reference binary. They execute proc.test
and rename.test without backend overlays. Frame-debug settings and skipped
constraints remain explicit in each capture. These C results do not establish
Runtime/VM or migrated-consumer equivalence.

| Reference | Capture | Passed | Skipped | Failed |
| --- | --- | --- | --- | --- |
| C8.4.20 | [Original inputs and results](results/original-suite-integrity-c84-20261013/README.md) | 30 | 0 | 0 |
| C8.5.19 | [Original inputs and results](results/original-suite-integrity-c85-20261013/README.md) | 36 | 16 | 0 |
| C8.6.18 | [Original inputs and results](results/original-suite-integrity-c86-20261013/README.md) | 40 | 17 | 0 |
| C9.0.4 | [Original inputs and results](results/original-suite-integrity-c90-20261013/README.md) | 40 | 17 | 0 |
| C9.1.0 | [Original inputs and results](results/original-suite-integrity-c91-20261013/README.md) | 40 | 17 | 0 |

## Run original naming suites

Build the VM `run_test` and runtime `run_script` examples into separate Cargo
targets. For the runtime, set `TCL_TOMMATH_DIR` to the matching provisioned
libtommath source before building. Use the source-built C executable directly;
the installer convenience wrapper deliberately unsets `TCL_LIBRARY`.

```sh
python3 experiments/name-handling/run_suites.py \
  --tcl-root tmp/tcl9.0.4 --tclsh tmp/tcl9.0.4/unix/tclsh \
  --vm /absolute/vm-target/debug/examples/run_test \
  --runtime /absolute/runtime-target/debug/examples/run_script \
  --output /absolute/new-capture-directory
```

The VM driver is currently fixed to Tcl 9.0. Runtime-only matrices may use
`--version` with a matching source tree/reference. Add `--no-overlay` for Tcl
8.4 because the shared backend overlay requires 8.5+ syntax. Use repeated
`--stem` options and `--match` for exact original-case isolation. Capture exits
indicate recording completion; inspect the records and actual Tcltest counts
before making a conformance claim. The tool enables frame debugging on C/runtime
and records it; absolute-frame-depth/default-debug upstream expectations still
need launch-context interpretation. Use a new output directory for each run.
