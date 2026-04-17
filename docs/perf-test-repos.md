# Performance Test Repositories

External Tcl projects used for benchmarking semantic token delivery time.
These repos are **not** included in the tcl-lsp repository — clone them
separately to run benchmarks.

## Running benchmarks

**Wall-clock benchmark** (measures open-to-tokens latency with an LSP client):

```bash
# Single file
python3 scripts/perf_semantic_tokens.py /path/to/practcl.tcl

# With simulated network latency (10 ms one-way)
python3 scripts/perf_semantic_tokens.py --latency-ms 10 /path/to/practcl.tcl

# All .tcl files in a directory (sorted by size, limited to 50)
python3 scripts/perf_semantic_tokens.py /path/to/tcllib/modules/

# JSON output for CI
python3 scripts/perf_semantic_tokens.py --json /path/to/practcl.tcl
```

**Profiling** (per-phase CPU breakdown with cProfile):

```bash
# Full breakdown
python3 scripts/profile_semantic_tokens.py /path/to/practcl.tcl

# Focus on a specific phase
python3 scripts/profile_semantic_tokens.py --phase tokens /path/to/practcl.tcl

# Dump profile for snakeviz
python3 scripts/profile_semantic_tokens.py --dump /tmp/prof.pstats /path/to/practcl.tcl
```

## General Tcl

### tcltk/tcllib

Tcl standard library — hundreds of packages with deep namespace and
`package require` chains.

- **URL:** <https://github.com/tcltk/tcllib>
- **`cloc` (Tcl/Tk only):** 923 files, 56 987 blank, 82 953 comment, 325 376 code

| File | Lines | Purpose |
|------|------:|---------|
| `modules/fumagic/filetypes.tcl` | 85 041 | Largest Tcl file; auto-generated file-type magic |
| `modules/practcl/practcl.tcl` | 8 463 | Largest hand-written; 114 procs, 212 chunks |
| `modules/struct/graphops.tcl` | 2 274 | 63 `package require`/`source` calls |
| `modules/amazon-s3/S3.tcl` | 3 117 | 57 `package require`/`source` calls |

**Baseline timings (0 ms latency):**

| File | Open-to-tokens | `workspace_state.update` | `semantic_tokens_full` | Tokens |
|------|---------------:|-------------------------:|-----------------------:|-------:|
| `practcl.tcl` (8 463 lines) | 34 502 ms | 32 406 ms | 1 874 ms | 13 752 |

**`practcl.tcl` timing breakdown:**

```
tokenise=109ms  compile=1087ms  analyse=1564ms  chunk_caches=29577ms  total=32338ms (procs=114)
semantic_tokens_full 1874ms (collect=1017ms, encode=857ms, tokens=13752, lines=8464)
```

### tcltk/tklib

Tk standard library — GUI package collection.

- **URL:** <https://github.com/tcltk/tklib>

### aplsimple/alited

Tcl/Tk editor — large multi-file Tcl/Tk application.

- **URL:** <https://github.com/aplsimple/alited>

### openocd-org/openocd

On-chip debugger with ~100+ Tcl target and configuration scripts.

- **URL:** <https://github.com/openocd-org/openocd>

## From Issue #31

### OSVVM/OSVVM-Scripts

FPGA verification framework with many well-commented procs and multi-line
docstrings.

- **URL:** <https://github.com/OSVVM/OSVVM-Scripts>
- **`cloc` (Tcl/Tk only):** 57 files, 1 877 blank, 5 138 comment, 7 161 code

| File | Lines | Purpose |
|------|------:|---------|
| `OsvvmScriptsCore.tcl` | 2 322 | Largest file; 128 procs |
| `StartUpShared.tcl` | 204 | Most `source` calls (26) |

**Baseline timings (0 ms latency):**

| File | Open-to-tokens | `workspace_state.update` | `semantic_tokens_full` | Tokens |
|------|---------------:|-------------------------:|-----------------------:|-------:|
| `OsvvmScriptsCore.tcl` (2 322 lines) | 2 898 ms | 2 204 ms | 576 ms | 4 769 |
| `StartUpShared.tcl` (204 lines) | 280 ms | 148 ms | 23 ms | 327 |

**`OsvvmScriptsCore.tcl` timing breakdown:**

```
tokenise=19ms  compile=637ms  analyse=875ms  chunk_caches=654ms  total=2184ms (procs=128)
semantic_tokens_full 576ms (collect=572ms, encode=3ms, tokens=4769, lines=2323)
```

### Hog (CERN)

CERN FPGA project management — Doxygen-style docstrings, real EDA workflows.

- **URL:** <https://gitlab.com/hog-cern/Hog>

## EDA / FPGA — Dialect-Specific

Each should be tested with its native dialect to exercise dialect command loading.

### Xilinx/XilinxTclStore — `xilinx-eda-tcl`

Official Vivado Tcl app repository.

- **URL:** <https://github.com/Xilinx/XilinxTclStore>
- **`cloc` (Tcl/Tk only):** 293 files, 18 051 blank, 23 608 comment, 89 368 code

| File | Lines | Purpose |
|------|------:|---------|
| `tclapp/icl/protoip/make_template.tcl` | 11 193 | Largest file (note: contains Latin-1 `®` byte) |
| `tclapp/xilinx/designutils/prettyTable.tcl` | 7 982 | Large file; 109 procs, 117 chunks |
| `tclapp/xilinx/projutils/write_project_tcl.tcl` | 3 379 | 60 `package require`/`source` calls |

**Baseline timings (0 ms latency):**

| File | Open-to-tokens | `workspace_state.update` | `semantic_tokens_full` | Tokens |
|------|---------------:|-------------------------:|-----------------------:|-------:|
| `prettyTable.tcl` (7 982 lines) | 40 541 ms | 37 500 ms | 2 828 ms | 21 693 |

**`prettyTable.tcl` timing breakdown:**

```
tokenise=123ms  compile=2172ms  analyse=5443ms  chunk_caches=29677ms  total=37415ms (procs=109)
semantic_tokens_full 2828ms (collect=1966ms, encode=862ms, tokens=21693, lines=7983)
```

### hukenovs/tcl_for_fpga — `xilinx-eda-tcl`

Vivado project creation, IP generation, timing analysis.

- **URL:** <https://github.com/hukenovs/tcl_for_fpga>

### Digilent/digilent-vivado-scripts — `xilinx-eda-tcl`

Production FPGA board design flows.

- **URL:** <https://github.com/Digilent/digilent-vivado-scripts>

### The-OpenROAD-Project/OpenROAD-flow-scripts — `synopsys-eda-tcl`

RTL-to-GDS autonomous flow used in 600+ silicon tapeouts.

- **URL:** <https://github.com/The-OpenROAD-Project/OpenROAD-flow-scripts>
- **`cloc` (Tcl/Tk only):** 137 files, 862 blank, 676 comment, 4 840 code

| File | Lines | Purpose |
|------|------:|---------|
| `flow/util/cell-veneer/lefdef.tcl` | 978 | Largest file |
| `flow/scripts/floorplan_to_place.tcl` | 97 | Most `source` calls (10) |

### The-OpenROAD-Project/OpenLane — `synopsys-eda-tcl`

`flow.tcl` orchestration with SDC constraints.

- **URL:** <https://github.com/The-OpenROAD-Project/OpenLane>

### mflowgen/mflowgen — `cadence-eda-tcl`

Modular ASIC flow with Cadence Innovus scripts.

- **URL:** <https://github.com/mflowgen/mflowgen>

### StanfordVLSI/dragonphy2 — `cadence-eda-tcl`

Innovus P&R scripts for analog/mixed-signal design.

- **URL:** <https://github.com/StanfordVLSI/dragonphy2>

### pConst/quartus_design_space_explorer_template — `intel-quartus-eda-tcl`

Quartus iterative compilation and reporting.

- **URL:** <https://github.com/pConst/quartus_design_space_explorer_template>

### paulscherrerinstitute/PsiSim — `mentor-eda-tcl`

ModelSim/Questa regression testing framework.

- **URL:** <https://github.com/paulscherrerinstitute/PsiSim>

## F5 iRules

### f5devcentral/irules-toolbox

Official F5 collection of production and mitigation iRules.

- **URL:** <https://github.com/f5devcentral/irules-toolbox>
- **`cloc` (Tcl/Tk only):** 2 files, 85 blank, 268 comment, 245 code

| File | Lines | Purpose |
|------|------:|---------|
| `security/http/cookies/samesite-attributes-pre-v12.tcl` | 317 | Largest iRule |

**Baseline timings (0 ms latency):**

| File | Open-to-tokens | `workspace_state.update` | `semantic_tokens_full` | Tokens |
|------|---------------:|-------------------------:|-----------------------:|-------:|
| `samesite-attributes-pre-v12.tcl` (317 lines) | 288 ms | 152 ms | 69 ms | 520 |

### simonkowallik/F5-iRules

Community collection of complex production iRules.

- **URL:** <https://github.com/simonkowallik/F5-iRules>

### f5devcentral/f5-agility-labs-irules

Advanced iRule lab exercises with complex rules.

- **URL:** <https://github.com/f5devcentral/f5-agility-labs-irules>

### landro/TesTcl

iRule unit testing framework with test fixture iRules.

- **URL:** <https://github.com/landro/TesTcl>

## Latency Impact Summary

Tested with `samples/for_screenshots/16-references-long.tcl` (351 lines):

| Latency | Open-to-tokens | Overhead vs 0 ms |
|--------:|---------------:|------------------:|
| 0 ms | 161 ms | — |
| 10 ms | 225 ms | +64 ms |

The ~64 ms overhead at 10 ms one-way latency matches the expected 3 round
trips × 20 ms (send + receive) = 60 ms.  At intercontinental latencies
(50–100 ms one-way) this overhead would be 300–600 ms.
