# mro_eval corpus manifest

The corpus for the TclOO class-lattice experiment
(`experiments/mro_eval/RESULTS.md`). External sources are **git-ignored**
(see `.gitignore`) and reconstituted by `fetch_corpus.sh`; only the
hand-written `adversarial/` snippets and this manifest are committed.

Totals used in RESULTS.md: **154 files, 66,827 lines, 448 TclOO classes,
1,803 `$obj method` sites**.

## Committed (in-tree)

| path | provenance |
|---|---|
| `adversarial/dynamic_dispatch.tcl` | hand-written for this experiment; every `$obj method` must abstain (⊤). Mirrored by `rust/tcl-compiler/tests/mro_lattice_adversarial.rs`. |

## Git-ignored (fetched by `fetch_corpus.sh`)

### `georgtree/` — real OSS TclOO (github.com/georgtree), shallow clones

| repo | commit | .tcl files | lines | notes |
|---|---|--:|--:|---|
| SpiceGenTcl | d9d7187 | 59 | 16,428 | flagship TclOO project (`tcloo`-tagged); SPICE netlist generator |
| tclopt | ff9a560 | 11 | 4,811 | non-linear fitting (Mpfit), TclOO wrappers |
| ruff | b63da69 | 11 | 7,703 | doc generator (fork of apnadkarni/ruff), TclOO |
| argparse | 139d695 | 10 | 2,697 | argument parser |
| tcl_tools | 6be5470 | 10 | 2,078 | misc utilities |
| tclinterp | ccd894b | 6 | 934 | interpolation wrappers |
| tclmeasure | 648b0cc | 3 | 529 | SPICE measure |
| extexpr | 1d53317 | 3 | 75 | expr extensions |

### `vendor/tcllib/` — tcllib 2.0 metaobject frameworks

`clay.tcl`, `oodialect.tcl`, `oometa.tcl`, `oooption.tcl`, `ooutil.tcl`,
`pkgIndex.tcl` — copied from `tmp/tcllib-2.0/modules/{clay,oodialect,
oometa,ooutil}` (session-provided tcllib tree).

### `vendor/tcl-oo-tests/` — Tcl core `oo` test suites

`oo.test`, `ooNext2.test`, `ooUtil.test` from the Tcl **8.6.16** and
**9.0.3** source trees (`tests/`), copied and prefixed with the version.
Real TclOO exercised by the interpreter's own conformance tests.

### `repo-fixtures/` — this repo's analyser OO fixtures

`oo-shapes.tcl`, `methodParam.tcl` from `editors/vscode/testFixture/`.
