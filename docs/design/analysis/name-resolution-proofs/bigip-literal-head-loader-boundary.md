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

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim

Status: `not-tested`.

No independent C Tcl or Jim execution of this appliance question is claimed.

### bigip

Status: `observed`. Channel: Canonical load channels and exact rejection logs; no runtime answer is granted to a source that did not load. Version: bigip 21.1.0.1.

The canonical53-source set has40acceptances and13rejections, with no warnings/indeterminate loads. Literal namespace/interp/package/rename and ordinary procedure-call doors have explicit loader errors; their dynamic controls are different inputs. Isolated later LF/CRLF and NUL controls do not retroactively repair rejected canonical sources.

The measured report rows and their limits are:

The canonical set produced 40 `ACCEPT`, 13 `REJECT`, no warnings, and no indeterminate loads. Complete status and per-rule logs are `load-driver-summary.tsv` and journals.

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

The raw edge-control evidence, including exact `.conf.hex` files and the literal-NUL loader bytes, is under r2286_20261006de.

## Consumer bindings

No implementation binding is claimed by this observation record.

## Evidence

- [scripts/dev/bigip-probes/resolution-2286/BIGIP_RESULTS.md](../../../../scripts/dev/bigip-probes/resolution-2286/BIGIP_RESULTS.md)
