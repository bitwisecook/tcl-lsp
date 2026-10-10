# naming.bigip.dynamic-resolution-controls

Kind: `native-observation`

## Problem statement

A literal command surface may reject an iRule before a dynamic command is reached. Alias/rename/import and traced-cell controls can also leave state that changes a repeat. Treating all rows as ordinary Tcl runtime availability would erase these distinctions.

## Question

What did each byte-retained dynamic resolution program return on first and repeated execution?

## Conclusion

The table retains each first result and state-dependent repeat, including unsupported namespace path, alias/rename collision errors, import reuse, trace registration retirement, counted names, caught read errors and expression failures. These are exact program outcomes; errorCode was unavailable and literal Unicode name programs never executed.

## Scope

The tabled dynamic sources, CMP-disabled and four-unit traffic; outer eval completion and inner caught errors are distinct. BIG-IP21.1.0.1 build0.0.26 Point Release1 only; appliance implementation source is unavailable.

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

Status: `observed`. Version: 21.1.0.1. Build: 0.0.26, Point Release1; no separate hotfix field reported. Channel: The tabled dynamic sources, CMP-disabled and four-unit traffic; outer eval completion and inner caught errors are distinct.. Dialect: F5 iRules and the explicitly recorded hosted contexts.

The table retains each first result and state-dependent repeat, including unsupported namespace path, alias/rename collision errors, import reuse, trace registration retirement, counted names, caught read errors and expression failures. These are exact program outcomes; errorCode was unavailable and literal Unicode name programs never executed.

The measured report rows and their limits are:


Every row below loaded without warning, ran first and repeated on the CMP-disabled TMM, was cleaned by exact generated names, then ran on units 0, 1, 2, and 3 with 32 responses per unit. Unless a repeat result is stated, the repeat matched the first result. Returned `errorCode` availability was `UNAVAILABLE`, including caught errors, so no error-code value is inferred.

| Case | First measured result |
| --- | --- |
| `dynamic_namespace` | `11` |
| `dynamic_path_shadow` | Error: `bad option "path": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which` |
| `dynamic_path_provider` | Same unsupported `namespace path` error |
| `dynamic_global_alias` | `11`; the proc's `global` alias reached the generated global cell |
| `dynamic_upvar` | `9` |
| `dynamic_upvar_unset` | `AGAIN` after target unset/recreate |
| `dynamic_uplevel` | `42` |
| `dynamic_upvar_absolute` | `CHANGED CHANGED` |
| `dynamic_upvar_zero` | `CHANGED` |
| `dynamic_uplevel_absolute` | `ABSOLUTE` |
| `dynamic_static_global_link` | `STATIC` |
| `dynamic_alias` | `ONE:FIXED` |
| `dynamic_alias_rename` | `1 {invalid command name "…_one"} ORIGINAL`; repeat failed earlier because `…_moved` already existed |
| `dynamic_rename_epoch` | `OLD NEW OLD`; repeat failed earlier because `…_moved` already existed |
| `dynamic_namespace_import` | `EXPORTED`; repeat: `can't import command "p": already exists` |
| `dynamic_namespace_delete` | `OLD NEW` |
| `dynamic_trace_scalar` | Read, write, unset callbacks in order; recreation did not retain the unset variable's trace |
| `dynamic_trace_upvar_array` | First `1 {}`; repeat recorded `{::…_arr k w}` from the pre-existing whole-array trace |
| `dynamic_trace_static` | Read then unset callbacks; recreation did not retain the unset cell's trace |
| `dynamic_package` | `8.4 0 8.4` (`provide Tcl`, require catch, required version) |
| `dynamic_package_resolution` | `1.0 1.0` |
| `dynamic_colon_names` | `7 7` for the tested multi-colon and canonical qualified spellings |
| `dynamic_read_failure` | Outer eval succeeded with value `1 {can't read "missing_r2286_20261006c": no such variable}` because the inner script caught the abrupt read |
| `dynamic_exists` | `0 0` without manufacturing contents |
| `dynamic_expr` | `3.5 14 3 1 {unknown math function "future_function"}` |
| `dynamic_word_bytes` | `{A B} {A B} {\n}` |

`dynamic_unicode_names` and `dynamic_unicode_variable_syntax` never ran: their outer iRules were rejected at load due to the literal Unicode bytes. This is a literal/dynamic boundary result, not evidence about runtime normalization or `$name` tokenization.


## Exact evidence

- `report-section` (observation): [scripts/dev/bigip-probes/resolution-2286/BIGIP_RESULTS.md](../../../../scripts/dev/bigip-probes/resolution-2286/BIGIP_RESULTS.md). SHA-256 `02a0c0e69ba8ed1ceee716fdca5be0c40bc0592bde086213c557b52c6d637136`. Lines 133–167. Exact build-specific measured report section; all rows and limits remain in the retained report.
- `provider-version` (provider): [scripts/dev/bigip-probes/resolution-2286/BIGIP_RESULTS.md](../../../../scripts/dev/bigip-probes/resolution-2286/BIGIP_RESULTS.md). SHA-256 `02a0c0e69ba8ed1ceee716fdca5be0c40bc0592bde086213c557b52c6d637136`. Lines 3–3. Actual appliance version attribution for this report; no separately named hotfix inferred.
- `source-closure-2` (provider): [scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006c/appliance/inventory.txt](../../../../scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006c/appliance/inventory.txt). SHA-256 `99cc929a22a600c11a700fff0b2dc1cb7333ed525bcdb87ab041c9a1187e45ec`. Retained report-specific source manifest or appliance inventory; individual measured inputs remain distinct.
- `source-closure-3` (input): [scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006c/appliance/manifest-verification.txt](../../../../scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006c/appliance/manifest-verification.txt). SHA-256 `f800077eaadc0104201639706c8d912d3804118dd6175be3f9cd727052d1bbbe`. Retained report-specific source manifest or appliance inventory; individual measured inputs remain distinct.
- `source-closure-4` (input): [scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006hi/appliance/manifest.json](../../../../scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006hi/appliance/manifest.json). SHA-256 `8f5c9f385b226b5f7da6885ede6a28775a68cdfea7ed44fb28fb213c73f6e6fd`. Retained report-specific source manifest or appliance inventory; individual measured inputs remain distinct.
- `aggregate-5` (observation): [scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006c/result.json](../../../../scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006c/result.json). SHA-256 `ff299fa606867041acd7975db9d13cca40e5ba7b6135f8228d22168d5388d725`. Inspected measured canonical case aggregate and its explicit unmeasured limits; actual case rows remain in this exact file.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

## Replay

The pinned report contains the exact source-generation, load and traffic protocol. This record claims no new appliance execution; rerun requires the recorded isolated appliance context, exact bytes and independently captured load/event outcomes. Closed implementation source and unmeasured releases are unavailable.
