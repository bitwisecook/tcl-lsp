# Why is the residual fixture's `cu.function("::p")` missing?

Current-source diagnosis: the fixture supplies an unprofiled catalogue without
a complete source metadata context. Analysis lowering declines to collect the
procedure before name publication. The evidence does not support replacing
`::p` with a display-name fallback or declaring the Tcl source invalid.

The retained run compiled, then all six controls failed at the same procedure
lookup before SCCP or their widening/constant assertions. See
`experiments/name-handling/results/fixture-source-input/20261012T171419Z/README.md`.
This review adds no execution or counterfactual result.

| Current owner | Relevant boundary |
| --- | --- |
| `compiler_analysis_residual.rs:88,2035` | `reg()` calls `CommandRegistry::build_default()`; the helper calls `CompilationUnit::build_for`, then exact `function("::p")`. The file's `D = "tcl8.6"` is unused by this helper. |
| `tcl-registry/src/registry.rs:1871,2144` | The default catalogue stores `profile: None`; `profile()` returns that stored value. Catalogue membership supplies no selected source profile. |
| `compilation_unit.rs:2962,2986,2430` | `build_for` supplies default lexer config, `dialect: None`, and no entry/context/input. `lower_and_build_cfg` forwards those absences; it cannot construct a context input without a context and dialect. |
| `lowering/mod.rs:1279,1363,1560` | The target is Analysis. `with_dialect(None)` supplies no dialect context; module initialisation retains no source input when both dialect and registry profile are absent. |
| `lowering/mod.rs:4968,3236,3516` | `invocation_metadata_context()` returns `None` on this path. Logical procedure extraction requires that context and its source input. Subsequent logical structured-command routing also requires the context, so it returns the generic call before invoking the proc lowering hook. |
| `lowering/mod.rs:3421` | The independent rule-loader declaration route requires an iRules profile; this unprofiled registry does not enter it. |
| `cfg_builder/mod.rs:3590`; `compilation_unit.rs:2755,3970` | CFG and function collection follow the IR procedure map. `function()` checks `::top`, then performs `self.procedures.get(name)`; it does not reinterpret display names. |

The driver does separately select a documented C9.0 invocation recipe for an
unprofiled catalogue (`environment_ingress.rs:39`). That recipe does **not**
fill the missing source/profile metadata context. It must not be used to
manufacture one or assert a native registration/execution entry.

The failure is therefore a fixture/driver ownership mismatch under the current
policy, expressed as withheld declaration inventory. It is not evidence that
`p` was collected under a different key. Whether this mismatch is pre-existing,
or whether the compatibility entry's documented dialect-blind behaviour needs
revision, remains unknown without a controlled comparison.

The next narrow test should keep the same safe-call source and compare this
exact baseline with an explicitly selected C9.0 registry and
`CompilationUnit::build_for_dialect(..., "tcl9.0")`. Before SCCP, record the
registry profile, retained module source input, IR/CFG/CU procedure keys, and
the top-level declaration's residual. Assert the exact intended key only when
the selected owner actually collects it; retain the baseline's absence.
Then run the handler, conditional-handler and safe-call assertions under the
reviewed input, preserving `fu.source_metadata_input()`. A selected context
supplies conditional source advice, not actual handler execution or oracle
authority. Do not replace collection with a synthetic function, name fallback,
or permissive declaration. No source edits, Cargo runs or Git mutations were
performed in this review.
