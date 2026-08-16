# Differential fuzzing contracts

How the differential fuzzer hunts for miscompiles, and the discipline that
keeps a campaign honest: a run that reports generator artefacts as
divergences, masks a real miscompile, or produces a finding nobody can replay
is worse than no run at all.

`rust/tcl-fuzz` generates Tcl scripts, runs each one through a **pair of
backends**, and records any divergence in stdout, error status, or (opt-in)
error message text. Everything is seeded, so any finding replays exactly.

Four arms exist, each answering a different question:

| Arm | Module | Compares |
|---|---|---|
| pairwise differential | `campaign.rs` | any two `Engine`s over the same subprocess harness — `tclvm`/`tclsh` (the default), `runtime-rust`/`tclsh`, or `tclvm`/`runtime-rust` |
| three-way characterisation | `characterize.rs` | the C Tcl 9 oracle against **both** native implementations at once |
| WASM value differential | `wasm_diff.rs`, `linked_wasm.rs` | the WASM emitter's structured control flow against `tcl-vm`'s normal execution |
| eBPF value differential | `bpf_diff.rs` | the BPF-Tcl lowering + eBPF emitter + userspace eBPF VM against the declared contract |

## Decision rules / contracts

1. **The generator emits only valid, pure, bounded Tcl.** Every script is
   syntactically valid (balanced `{}` / `[]` / `""`), has no I/O, file, socket,
   `exec`, `clock`, or `after` command, uses literal integer loop bounds so
   neither backend can hang, and prints deterministic output through `puts` so
   the differential has something to compare. There is deliberately no
   malformed-input mode: a divergence must point at a real miscompile, not at
   error-recovery wording.
2. **Generation is scoped to the surface the subject backend implements**, for
   the same reason — an unimplemented command is not a finding.
3. **A finding is keyed by its seed.** `GenConfig` plus the seed reproduce the
   script byte-for-byte; the registry stores the JSON record and the raw
   `.tcl` beside it, de-duplicates by seed, and summarises by category.
4. **A Tcl release is a pair-wide campaign fact.** `--tcl-version` is parsed
   through `tcl_dialect::TclVersion`, normalised to its `major.minor` line,
   and applied to every engine that emulates a release at run time. A
   fixed-release engine such as C `tclsh` receives no invented flag; its
   `[info patchlevel]` probe must instead report the requested line. Any
   engine that cannot honour the requested release stops the command before
   the campaign starts, rather than silently running a skewed pair.
5. **Finding identity includes the selected release.** An unpinned legacy
   pair retains its existing path for compatibility. A pinned pair adds a
   `tclX.Y` child below its pair directory, so the same pair and seed at Tcl
   8.6 and Tcl 9.0 cannot de-duplicate or overwrite each other. The canonical
   campaign release is serialised with every finding, separately from observed
   backend patchlevels. A replay-metadata schema marker distinguishes a newly
   recorded deliberate unpinned campaign from an ambiguous older no-release
   record.
6. **Replay restores or refuses the recorded configuration.** With no
   explicit release, one release-aware record is discovered from the pair's
   `tclX.Y` registries and replayed under that release. Multiple matching
   releases require an explicit `--tcl-version`, which selects its exact
   release namespace; absent selected records, inconsistent metadata, and
   legacy no-release records lacking the schema marker are refused. A marked
   unpinned record replays in the plain pair namespace. `summary` discovers
   every existing pair namespace and labels each release separately; an
   explicit release filters to its exact namespace. Replaying at a current default with
   only a warning is not a safe reproduction. Once the release configuration
   is restored, differing recorded versus current backend patchlevels remain a
   warning: they are build provenance, not a substitute for the release check.
7. **Comparison rules** (`harness::compare_outcomes`) are fixed:
   - either side unavailable → **skipped**;
   - a *reference* timeout means the script is pathological → **skipped**,
     never blamed on the subject; a *subject* timeout is a finding;
   - differing error status → `StatusMismatch`;
   - stdout is compared **only when neither side errored** — an errored run's
     partial stdout is not meaningfully comparable;
   - error *text* is compared only when explicitly enabled
     (`ErrorTextMismatch`), because C Tcl's wording is a separate conformance
     question from behaviour.
8. **A two-way native pair has no oracle.** `tcl-vm` ↔ `runtime/rust` detects
   drift but both backends can agree on the same bug, so the three-way
   characterisation classifies every run against C Tcl 9: only `tcl-vm`
   diverges, only `runtime/rust` diverges, both agree with each other but
   differ from C (the shared-bug case a pair misses), or a genuine three-way
   disagreement.
9. **The WASM arm isolates one variable.** Both sides evaluate the actual Tcl
   commands with `tcl-vm`, so command semantics are held constant and the only
   difference is whether control flow comes from the WASM emitter's structured
   codegen or from normal execution. A divergence therefore isolates a WASM
   control-flow miscompile, with no `tclsh` involved and no confounding from
   `tcl-vm` command bugs.
10. **The eBPF arm is registry-driven.** Command spellings and the event name
   come from the registry's typed BPF descriptors, so changing a spelling in a
   spec changes the generated program with no fuzzer edit. C Tcl is not an
   oracle for BPF-Tcl, which is its own dialect.

## File-path anchors

- `rust/tcl-fuzz/src/generator.rs` — the grammar-aware generator and `GenConfig`.
- `rust/tcl-fuzz/src/engine.rs` — the `Engine` enum (`tclvm`, `tclsh`,
  `runtime-rust`).
- `rust/tcl-fuzz/src/harness.rs` — subprocess execution, `Outcome`, `Verdict`,
  `compare_outcomes`.
- `rust/tcl-fuzz/src/campaign.rs` — generate → run → compare → record.
- `rust/tcl-fuzz/src/characterize.rs` — three-way `Classification`.
- `rust/tcl-fuzz/src/findings.rs` — the seed-keyed registry and `Category`.
- `rust/tcl-fuzz/src/wasm_diff.rs`, `linked_wasm.rs`, `wasm.rs` — the WASM arm.
- `rust/tcl-fuzz/src/bpf_diff.rs` — the eBPF arm.
- `rust/tcl-fuzz/src/rng.rs` — the seeded RNG every arm shares.

## Failure modes

- A generator change silently narrowing the surface, so a whole command family
  stops being exercised while the campaign still reports "clean".
- Comparing stdout across an errored run and reporting the partial output as a
  divergence.
- Treating a shared `tcl-vm` + `runtime/rust` bug as agreement, which is
  exactly what the three-way arm exists to prevent.
- A finding recorded without its seed, making it unreplayable.

## Discoverability

- [Design doc index](../README.md)
- The `fuzz-findings` skill drives campaigns, summarises the registry, and
  replays a seed.
- [VM/bytecode test boundary](vm-bytecode-test-boundary.md)
