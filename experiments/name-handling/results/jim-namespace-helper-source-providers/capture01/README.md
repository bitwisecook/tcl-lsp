# Original Jim namespace-helper source providers

The original SDK loads nine helper procedures through `Jim_InitStaticExtensions`.
Neither `Jim_RegisterCoreCommands` alone nor adding only `Jim_namespaceInit`
loads them. Loading the original `nshelper.tcl` file is a separate source purpose.

This capture observed the original public `Jim_Cmd` procedure objects and
`Jim_GetSourceInfo`, without replacing a helper or changing provider files.
Fresh upstream HEAD resolved to `5bac7c99ad65864c87da513e22e2f01703fa4e03`
at 2026-10-15 11:14:47 UTC. The manifest retains the exact source, archive,
binary, configuration, observer, compiler commands and stdout/stderr hashes.
All seven subprocess stages completed with exit 0, the SDK emitted 211 rows,
and all 89 selected provider inputs remained unchanged. The independent
original namespace suite passed 79/79 with its original support files.

| Helper | Body first line | Static body bytes | Raw-file body bytes |
| --- | ---: | ---: | ---: |
| namespace delete | 7 | 224 | 224 |
| namespace origin | 17 | 309 | 309 |
| namespace which | 32 | 358 | 358 |
| namespace code | 49 | 138 | 156 |
| namespace inscope | 57 | 43 | 43 |
| namespace import | 61 | 585 | 696 |
| namespace info | 87 | 909 | 1059 |
| namespace upvar | 138 | 182 | 182 |
| namespace ensemble | 147 | 304 | 325 |

Static bodies identify `nshelper.tcl`; both raw-file controls identify the
actual absolute file path passed to `Jim_EvalFile`. Raw-only and
raw-after-static observations match byte for byte for all nine formals and
bodies. Four static bodies differ because the original `make-c-ext.tcl`
generator removes comment text while preserving lines. All body objects were
resident `source` primaries with one native reference. Formal objects were
resident `list` primaries; their source observation was the original empty
filename and line 1, independently of the body's location.

`probe.stdout` and `manifest.json` are immutable execution records.
`observations.json` and the three provider directories extract the measured
bytes without inventing expected results. `static-evaluated-source.tcl` decodes
the original `_nshelper.c` string argument to `Jim_EvalSource`;
`raw-nshelper-source.tcl` is an exact copy of the original source file.
`derive.py` checks all 27 body slices against observed bytes and first lines.
These derived offsets identify retained corpus slices; the SDK did not issue
a byte-offset receipt or a live procedure-binding token.

The source providers must remain distinct. A raw-source descriptor is not
evidence that a backend installed the exact static body or filename/line.
This capture does not establish Runtime or VM execution, behavior of the
helpers, final-free lifetimes, other extension configurations, C Tcl or F5
semantics, or a complete system toolchain closure.

One upstream metadata request used an invalid parameter field before execution;
`failed-metadata-setup.json` records that setup failure separately. It supplied
no semantic observation. The corrected request is retained in `upstream.json`.

To verify the retained package:

```sh
python3 experiments/name-handling/results/jim-namespace-helper-source-providers/capture01/verify.py
```

Reproduction requires a fresh capture directory containing the exact observer,
driver and a fresh upstream receipt. Run `capture.py --provider <original-tree>`,
then `derive.py --provider <same-tree>`. Never overwrite this capture's raw
files. `scripts/dev/ensure-jim-oracle.sh` owns provisioning; the captured build
configuration is retained in `provider-verification.json`.
