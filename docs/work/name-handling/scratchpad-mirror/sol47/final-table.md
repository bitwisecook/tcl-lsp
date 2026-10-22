### `tcl-compiler` lib module filters (base vs final)

| Filter | Base ok/total | Final ok/total | Gains | Regressions |
|---|---|---|---|---|
| `analyser::bounds_checks` | 55/68 | 55/68 | +0 | 0 |
| `analyser::diagnostics::fp` | 442/581 | 445/581 | +3 | 0 |
| `analyser::diagnostics::helpers` | 5/22 | 5/22 | +0 | 0 |
| `analyser::diagnostics::security` | 28/36 | 28/36 | +0 | 0 |
| `analyser::diagnostics::tests` | 579/830 | 580/830 | +1 | 0 |
| `analyser::per_item` | 54/68 | 54/68 | +0 | 0 |
| `cfg_builder` | 86/262 | 86/262 | +0 | 0 |
| `codegen` | 515/552 | 515/552 | +0 | 0 |
| `command_binding` | 860/1176 | 860/1176 | +0 | 0 |
| `compilation_unit` | 33/98 | 33/98 | +0 | 0 |
| `compile_service` | 16/48 | 20/48 | +4 | 0 |
| `dynamic_names` | 29/48 | 29/48 | +0 | 0 |
| `gvn` | 58/94 | 58/94 | +0 | 0 |
| `interprocedural` | 55/127 | 55/127 | +0 | 0 |
| `irules_checks` | 55/116 | 55/116 | +0 | 0 |
| `lowering` | 229/405 | 229/405 | +0 | 0 |
| `lowering_hooks` | 22/28 | 22/28 | +0 | 0 |
| `optimiser` | 339/558 | 339/558 | +0 | 0 |
| `realm` | 34/43 | 34/43 | +0 | 0 |
| `registry_invocation` | 207/287 | 207/287 | +0 | 0 |
| `sccp` | 123/202 | 123/202 | +0 | 0 |
| `shimmer` | 93/214 | 94/214 | +1 | 0 |
| `ssa` | 156/229 | 157/229 | +1 | 0 |
| `taint` | 43/62 | 43/60 | +0 | 0 |
| `word_expr` | 8/8 | 8/8 | +0 | 0 |
