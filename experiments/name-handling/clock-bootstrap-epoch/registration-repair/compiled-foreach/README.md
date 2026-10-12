# Procedure loop publication and native iterator ownership

Question: what does the unchanged procedure/foreach source do in each actual
stock C shell and pinned Jim, compared with the frozen native C8.6 core runner?

The exact [input](../compiled-foreach-input.tcl), SHA-256
`adbf44511437984ac055a50a009bf85591101864b471fe7a52068612829b5083`,
defines a procedure whose loop publishes add/format/scan procedures and then
sets a marker. It calls that procedure without appending a print, catch,
namespace setup, harness or verdict assertion.

Each [reference-shell capture](reference-shells/records.json) executes that
same file once in a separate fresh process. Each C shell performs stock startup
using its matching real `TCL_LIBRARY`. Jim is the maintained latest pinned
oracle at `5bac7c99ad65864c87da513e22e2f01703fa4e03`
(`0.84-9-g5bac7c9`), with stock embedded startup and static extensions;
no C init script is substituted. The maintained pin, actual binary hashes,
matching library inventories, source hashes, commands, timeout policy and
byte-exact streams are retained in that directory.
The exact [capture source used](reference-shells/capture-source.py) matches the
source hash in its records; the maintained capture tool may evolve separately.

| Actual stock shell | Exit | Raw outcome |
| --- | --- | --- |
| C8.4.20 | 1 | Unknown namespace when creating `::tcl::clock::add` |
| C8.5.19 | 0 | Empty stdout and stderr |
| C8.6.18 | 0 | Empty stdout and stderr |
| C9.0.4 | 0 | Empty stdout and stderr |
| C9.1.0 | 0 | Empty stdout and stderr |
| Pinned Jim | 0 | Empty stdout and stderr |

Script mode does not echo the final result. A zero exit establishes ordinary
process completion for this exact source; it does not supply a printed marker,
an internal object lifetime measurement, or a proof of another engine's
iterator protocol. C8.4's missing namespace prevents the first procedure
publication and the later set; that context failure supplies no iterator answer.
F5 is explicitly unmeasured. No C or Jim answer is projected onto F5.

The frozen native C8.6 core-only runner independently aborts with SIGABRT
(`-6`) and `realloc(): invalid pointer`: [record](../runtime-compiled-foreach-record.json),
[stdout](../runtime-compiled-foreach.stdout.txt),
[stderr](../runtime-compiled-foreach.stderr.txt). Its startup differs from the
stock reference shells: matching real init was refused in a separate control,
so this source runs on the native core before those library procedures exist.
Its clock map and reading-worker registration orders also differ from actual
C8.6; membership and deferred targets match, but physical hash/table-order
equivalence is unmeasured. That remaining bootstrap gap has no inferred
relationship to the allocator failure.
The native allocator failure remains a separate implementation question;
stock-shell completion does not erase that failure or establish its cause.

The earlier focused runtime test [panicked](../runtime-test-before-lazy-selection.log)
on eagerly indexing an absent local temporary. Lazy selection prevents that
index panic, but the same test then [aborted](../runtime-test-after-lazy-selection.log).
These attempt logs have no independent test-binary hash. The standalone abort
has its own exact binary receipt. Shared [source snapshots](../build-selected-sources.json)
and [build records](../build-records.json) retain provenance; no original capture
bytes are moved or rewritten by this page.

To capture the unchanged input in fresh actual reference processes:

```
python3 experiments/name-handling/clock-bootstrap-epoch/registration-repair/capture.py --question compiled-foreach --output-dir /tmp/name-compiled-foreach-replay-new
```

The required output destination must not already exist. The replay creates
new result files and hashes the binaries/libraries actually selected that time.
