# Per-point binding state cost

Plan [item 92](../plan.md) measures the cost of keeping a
`ModuleCommandBindings` at each source dispatch, continuation and lifecycle
point. The medium-file budgets are 10% added analysis time and 25 MB of
additional retained memory.

## Workloads

- **687 lines, full analysis, Tcl 9.0.** Keep the blank line between the two
  pinned Overrides package files, remove the `source` and `package provide`
  commands from the second, and put `set probe 1` immediately after its final
  brace. [Pinned inputs](../caching-design.md#measured-cost)
- **Scaling corpus, structure-only analysis, Tcl 8.6.** The same generated
  script and 100, 200, 400, 600 and 800 procedure counts as the
  [analysis-performance handover](analysis-performance.md#how-to-measure).

## Measurement

Use a new process per observation and warm the selected registry on one
small script before measuring. Both timing runs invoke the normal
`Analyser::analyse` entry point. The control preserves each point's identity,
frame and registry baseline but omits the point's changing world and skips
world joins at repeated points. Downstream analysis reads those tables.
The control can change answers and the work they trigger. Subtraction of the
control wall time therefore measures the cost of running this reduced
analysis. The separately timed snapshot clones and joins measure direct work
in the unchanged analysis. They exclude map copies triggered later by a
write to a world shared with an earlier point.

For memory, read the live Rust allocation bytes after each completed analysis
with its result held. Clear the source memo entries before reading to remove
any extra references they retain. In the ordinary run, keep the document
realm's binding history and release the rest of the result. Replace each
retained point world with an allocation-free clone of an empty world and
count the bytes released. Add the inline slots at vector capacity to
estimate what removing the field would save. The probe reports any shared
inventories it must leave intact. This measurement attributes allocations
also reached from other still-live owners to those owners. The earlier
230 MB caching figure was process RSS over a series of edits.

## Results

Measured on 24 November 2026 at `a918704d87` using Rust 1.100.0 on x86-64
Linux. Timings are medians of five alternating runs for Overrides and three
for each corpus size, with compiler opt-level 1 and registry opt-level 0.
The retained-memory columns measure the completed analysis in each mode.
Units are decimal MB.

| Input | Lines | Time with / without (s) | Direct clone and join (ms) | Retained with / without (MB) | Change (MB) |
| --- | ---: | ---: | ---: | ---: | ---: |
| Overrides | 687 | 2.47 / 4.74 | 1.82 | 33.91 / 27.90 | +6.01 |
| 100 procedures | 1004 | 2.08 / 2.29 | 0.58 | 91.20 / 84.60 | +6.60 |
| 200 procedures | 2004 | 4.14 / 4.53 | 1.17 | 188.53 / 175.34 | +13.19 |
| 400 procedures | 4004 | 8.76 / 9.38 | 2.46 | 402.77 / 376.40 | +26.37 |
| 600 procedures | 6004 | 13.67 / 18.34 | 3.54 | 661.44 / 621.89 | +39.55 |
| 800 procedures | 8004 | 19.87 / 21.59 | 5.37 | 909.83 / 857.10 | +52.73 |

**Item 92 passes both medium-file budgets.** Ordinary analysis took 2.47 s
against 4.74 s with the point worlds omitted. Direct snapshot work took
1.82 ms, about 0.07% of ordinary analysis. Ordinary analysis emitted
21 diagnostics and the omission control emitted 19. The wall times compare
those two workloads with their respective answers.

The completed medium result retained 6.01 MB more with point worlds. An
independent opt-level 0 compiler run kept the ordinary document realm after
releasing the rest of the result. The whole remaining allocation delta was
21.72 MB. Clearing 404 point worlds accounted for at least 0.79 MB of
that figure, including 0.25 MB of inline capacity. One shared inventory
stayed live, so the 0.79 MB is a lower bound. Even the whole remaining
realm and process allocation delta was below the 25 MB budget.

The corpus retained roughly 66 kB more per generated procedure with point
worlds across this range. At 800 procedures the difference reached 52.73 MB.
The 600-procedure omission timings ranged from 15.04 s to 33.10 s. All other
corpus rows also had faster ordinary medians. Phase 3 keeps the existing
representation and uses these measurements as its scaling baseline.

## Repository checks

With item 92 based on `9014fac90d`, `make rust-check` passed. `make prep-pr`
cleared formatting, code generation,
editor checks and installer tests. The host sent SIGKILL to the compiler
while building the `tcl-compiler` library test harness on both attempts,
including a serial retry. The Rust smoke tests remain unrun.

## Reproduction

The [probe patch](../patches/name-world-cost-probe.patch) adds temporary
instrumentation and a standalone driver. Apply it in a clean worktree at
`a918704d87` and use a private target from
`scripts/dev/agent-build-env.sh`. The driver takes an input path or a
procedure count. It has `with` and
`without` timing modes, `with-memory` and `without-memory` for the completed
result, and `memory` for the isolated realm measurement.
`ITEM92_STRUCTURE=1` selects structure-only analysis.

In that worktree, build and link the probe. The link helper uses Cargo's
artifact paths so it also works with the Rust 1.100 build directory layout.

```sh
source scripts/dev/agent-build-env.sh
CARGO_PROFILE_DEV_OPT_LEVEL=1 cargo build --locked -p tcl-compiler --lib \
    --config 'profile.dev.package.tcl-registry.opt-level=0' \
    --message-format=json-render-diagnostics > .probe-92/build.jsonl
python3 .probe-92/link.py
```

Download the two pinned sources into `.probe-92/input`, then assemble the
medium script and run the workloads.

```sh
cat .probe-92/input/cmdOverrides.tcl > .probe-92/input/medium.tcl
sed -e '/^source /d' -e '/^package provide /d' -e '${/^$/d;}' \
    .probe-92/input/overrides.tcl >> .probe-92/input/medium.tcl
printf 'set probe 1\n' >> .probe-92/input/medium.tcl
test "$(wc -l < .probe-92/input/medium.tcl)" -eq 687

for mode in with without with-memory without-memory; do
    .probe-92/run "$mode" .probe-92/input/medium.tcl
    for procedures in 100 200 400 600 800; do
        ITEM92_STRUCTURE=1 .probe-92/run "$mode" "$procedures"
    done
done
.probe-92/run memory .probe-92/input/medium.tcl
```
