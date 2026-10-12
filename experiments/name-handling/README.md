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
| [results/RESULTS.md](results/RESULTS.md) | Current VM/runtime original-suite comparison and measured gaps |
| [run_suites.py](run_suites.py) | Reusable original-suite capture across real C/VM/runtime; retains outputs and provenance |
| [results/README.md](results/README.md) | Provenance and record format; records distinguish measured, pending and unsupported |
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
vector runners use `available_tclshs()`, which skips missing/invalid binaries
and does not honour that variable. Explicitly preflight/assert all five
validated references until those runners use complete-matrix discovery.
A locally installed unvalidated interpreter may provide a clearly labelled exploratory measurement,
not a completed reference gate.

## Upstream test suites

C Tcl's upstream tests run through `tcltest` on the matching real C interpreter
are valid oracle evidence. Jim's equivalent upstream suite, run on the
identified Jim build, serves the same role. Reuse relevant upstream name,
namespace, variable, frame, alias and lifecycle cases before writing a bespoke
probe. Use each C release's matching test tree and harness; do not assume a
newer suite runs unchanged on an older interpreter.

Record the suite revision, test file and case ID, exact test definition,
support files/packages, constraints, invocation and raw harness transcript.
Keep pass/fail/skip counts and reasons: a constrained or unexecuted case does
not establish its expected behaviour. Preserve actual results and errors when
reducing an upstream test to a shared conformance vector.

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
