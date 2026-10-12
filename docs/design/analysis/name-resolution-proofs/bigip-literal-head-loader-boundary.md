# naming.bigip.literal-head-loader-boundary

Kind: `native-observation`

## Problem statement

A source consumer may infer command availability from a dynamic runtime success even when the literal iRule compiler rejects the whole source. Load and execution are separate observations.

## Question

Which exact canonical sources were accepted or rejected by the iRule loader, and which were never executed?

## Conclusion

The canonical53-source set has40acceptances and13rejections, with no warnings/indeterminate loads. Literal namespace/interp/package/rename and ordinary procedure-call doors have explicit loader errors; their dynamic controls are different inputs. Isolated later LF/CRLF and NUL controls do not retroactively repair rejected canonical sources.

## Scope

Canonical load channels and exact rejection logs; no runtime answer is granted to a source that did not load. BIG-IP21.1.0.1 build0.0.26 Point Release1 only; appliance implementation source is unavailable.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No independent C Tcl or Jim execution of this appliance question is claimed.

### tcl8.5

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No independent C Tcl or Jim execution of this appliance question is claimed.

### tcl8.6

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No independent C Tcl or Jim execution of this appliance question is claimed.

### tcl9.0

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No independent C Tcl or Jim execution of this appliance question is claimed.

### tcl9.1

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No independent C Tcl or Jim execution of this appliance question is claimed.

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Jim Tcl.

No independent C Tcl or Jim execution of this appliance question is claimed.

### bigip

Status: `observed`. Version: 21.1.0.1. Build: 0.0.26, Point Release1; no separate hotfix field reported. Channel: Canonical load channels and exact rejection logs; no runtime answer is granted to a source that did not load.. Dialect: F5 iRules and the explicitly recorded hosted contexts.

The canonical53-source set has40acceptances and13rejections, with no warnings/indeterminate loads. Literal namespace/interp/package/rename and ordinary procedure-call doors have explicit loader errors; their dynamic controls are different inputs. Isolated later LF/CRLF and NUL controls do not retroactively repair rejected canonical sources.

The measured report rows and their limits are:


The canonical set produced 40 `ACCEPT`, 13 `REJECT`, no warnings, and no indeterminate loads. Complete status and per-rule logs are [load-driver-summary.tsv](../../../../scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006c/appliance/load-driver-summary.tsv) and [journals](../../../../scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006c/appliance/journals).

| Cases | Verdict | Exact diagnostic class |
| --- | --- | --- |
| `minimal`, `identity`, `identity_group`, `init_identity`, `static`, `static_group`, `global` | ACCEPT | No warning |
| `collision_a`, `collision_b`, `collision_observer` | ACCEPT | No warning |
| `procedure_a`, `procedure_b`, `procedure_caller` | ACCEPT | No warning |
| All dynamic cases in the preceding table | ACCEPT | No warning |
| `event_frames` | ACCEPT | No warning |
| `encoding`, `dynamic_unicode_names`, `dynamic_unicode_variable_syntax`, `physical_words_lf`, `physical_words_crlf`, `runtime_cleanup` | REJECT | `braces are required around the expression` |
| `ordinary_literal` | REJECT | `undefined procedure: same` |
| `literal_namespace` | REJECT | `command is disabled: "namespace"` |
| `literal_interp` | REJECT | `command is disabled: "interp"` |
| `literal_package` | REJECT | `command is disabled: "package"` |
| `literal_rename` | REJECT | `command is disabled: "rename"` |
| `literal_trace` | REJECT | `wrong # args` for `trace info variable x` |
| `literal_proc` | REJECT | `command is not valid in the current scope` |

The isolated edge controls remove confounding source features:

| Control and source hash | Load/runtime result |
| --- | --- |
| ASCII binary control `ae43d52f…630b` | ACCEPT; returned hex `417c65` |
| Precomposed literal `3305392a…c4de` | REJECT; `braces are required around the expression` |
| Decomposed literal `4aae0373…812a` | REJECT; same diagnostic |
| Physical ASCII LF `c41c4375…297f` | ACCEPT; `{A B} {A B} {\n}` in both CMP modes |
| Physical ASCII CRLF `be4978b8…2ff5` | ACCEPT; identical runtime value in both CMP modes |
| Dynamic embedded NUL `7fd169e9…c3ca` | ACCEPT; decoded result `3 410042 0 VALUE 0 VALUE 1` on all four TMMs |
| Literal embedded NUL `59294325…126d` | REJECT; `Syntax Error … can't parse TCL script beginning with`; the diagnostic output stopped after the source bytes `set x "A` at the NUL boundary |

The raw edge-control evidence, including exact `.conf.hex` files and the literal-NUL loader bytes, is under [r2286_20261006de](../../../../scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006de).


## Exact evidence

- `report-section` (observation): [scripts/dev/bigip-probes/resolution-2286/BIGIP_RESULTS.md](../../../../scripts/dev/bigip-probes/resolution-2286/BIGIP_RESULTS.md). SHA-256 `02a0c0e69ba8ed1ceee716fdca5be0c40bc0592bde086213c557b52c6d637136`. Lines 168–201. Exact build-specific measured report section; all rows and limits remain in the retained report.
- `provider-version` (provider): [scripts/dev/bigip-probes/resolution-2286/BIGIP_RESULTS.md](../../../../scripts/dev/bigip-probes/resolution-2286/BIGIP_RESULTS.md). SHA-256 `02a0c0e69ba8ed1ceee716fdca5be0c40bc0592bde086213c557b52c6d637136`. Lines 3–3. Actual appliance version attribution for this report; no separately named hotfix inferred.
- `raw-0` (observation): [scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006c/appliance/load-driver-summary.tsv](../../../../scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006c/appliance/load-driver-summary.tsv). SHA-256 `2a506d25eff955df31dac51386914d4c8815a4b37f4dc432f548145fb836846e`. Exact source or raw evidence explicitly linked by the measured section; its own context/channel remains authoritative.
- `source-closure-3` (provider): [scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006c/appliance/inventory.txt](../../../../scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006c/appliance/inventory.txt). SHA-256 `99cc929a22a600c11a700fff0b2dc1cb7333ed525bcdb87ab041c9a1187e45ec`. Retained report-specific source manifest or appliance inventory; individual measured inputs remain distinct.
- `source-closure-4` (input): [scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006c/appliance/manifest-verification.txt](../../../../scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006c/appliance/manifest-verification.txt). SHA-256 `f800077eaadc0104201639706c8d912d3804118dd6175be3f9cd727052d1bbbe`. Retained report-specific source manifest or appliance inventory; individual measured inputs remain distinct.
- `source-closure-5` (input): [scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006hi/appliance/manifest.json](../../../../scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006hi/appliance/manifest.json). SHA-256 `8f5c9f385b226b5f7da6885ede6a28775a68cdfea7ed44fb28fb213c73f6e6fd`. Retained report-specific source manifest or appliance inventory; individual measured inputs remain distinct.
- `aggregate-6` (observation): [scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006c/result.json](../../../../scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006c/result.json). SHA-256 `ff299fa606867041acd7975db9d13cca40e5ba7b6135f8228d22168d5388d725`. Inspected measured canonical case aggregate and its explicit unmeasured limits; actual case rows remain in this exact file.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

## Replay

The pinned report contains the exact source-generation, load and traffic protocol. This record claims no new appliance execution; rerun requires the recorded isolated appliance context, exact bytes and independently captured load/event outcomes. Closed implementation source and unmeasured releases are unavailable.
