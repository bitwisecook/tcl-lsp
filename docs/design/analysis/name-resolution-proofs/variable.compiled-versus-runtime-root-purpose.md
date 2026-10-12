# naming.variable.compiled-versus-runtime-root-purpose

Kind: `native-observation`

## Problem statement

Counted formal names can agree before raw NUL while retaining different bytes or lengths after it. A compiled lexical read and a dynamic variable-name object can then select different locals. Treating the display or the source-word shape as a universal cell key would merge those routes. Applicability depends on the original formal-list objects, counted procedure source and the selected engine; these controls do not authenticate an arbitrary current frame.

## Question

How do the retained nine original-name cases distinguish compiled braced substitution from dynamic original-object local lookup?

## Conclusion

The measured C compiled-name comparator and dynamic lookup are distinct purposes. C8.4/8.5 also parse formal lists through a CString extent. C8.6/C9 preserve counted formal names: equal-count raw-NUL names can select the first compiled primary while their dynamic cells remain distinct. Jim retains counted local lookup. The exact 216 rows, including definition errors, govern each case; no byte table or activation token is issued by this observation.

## Scope

Nine case names, one/two original-formal axes, definition/invocation rows; counted public C/Jim source/object inputs. Raw 00, encoded zero, post-zero qualifiers/array spelling, FF and duplicate names are separate controls. No Unicode file decoding, BIG-IP, general observer closure, Rust pass, source renaming or physical local-frame authority is inferred.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: linked library SHA d71ed42efd90354cdc99f73ba39b30e2cdedda463768a7fd025114b50db60011. Channel: original counted objects and procedure body via native API. Dialect: Tcl.

In raw-NUL equal-count one-formal control both compiled reads fail, while both dynamic reads find ONE; two formals fail arity after old list extent. Full case answers are reproduced below.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: linked library SHA 99e0d524e54498713498e43d6cf80fa085471913cb38a53b505e18c0469417df. Channel: original counted objects and procedure body via native API. Dialect: Tcl.

In raw-NUL equal-count one-formal control both compiled reads fail, while both dynamic reads find ONE; two formals fail arity after old list extent. Full case answers are reproduced below.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: linked library SHA a3a8abdeadd8aafa007d3bc9fa199b9cf48d43d01735ff41b3fcf33abb4019d6. Channel: original counted objects and procedure body via native API. Dialect: Tcl.

Equal-count raw-NUL names share first compiled read ONE; dynamic second name is missing with one formal and TWO with two. Full case answers are reproduced below.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: linked library SHA 2ba08ecf7197e16d99e303c0d95dde29c59f391a22fd02378edc2697567e0e04. Channel: original counted objects and procedure body via native API. Dialect: Tcl.

Equal-count raw-NUL names share first compiled read ONE; dynamic second name is missing with one formal and TWO with two. Full case answers are reproduced below.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: linked library SHA 8b2dba836908287f95f26a442bb065601daac300fdeee48b15e64a5c03894e33. Channel: original counted objects and procedure body via native API. Dialect: Tcl.

Equal-count raw-NUL names share first compiled read ONE; dynamic second name is missing with one formal and TWO with two. Full case answers are reproduced below.

### jim

Status: `observed`. Version: Jim; patch/revision not recorded in this manifest. Build: linked library SHA 75c74eef0c9d1ff2ff8972e6339678564a3439d6332f66032eb4db79004937ff. Channel: original counted objects and procedure body via native API. Dialect: Jim Tcl.

Equal-count raw-NUL names remain distinct in both compiled-source and dynamic lookup: ONE/missing with one formal, ONE/TWO with two. Full case answers are reproduced below.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No appliance observation for this question.

## Exact evidence

- `manifest` (provider): [rust/tcl-vm/tests/data/native_formal_slots/compiled-local-names-manifest.json](../../../../rust/tcl-vm/tests/data/native_formal_slots/compiled-local-names-manifest.json). SHA-256 `ab9f2482c437c47c5d4ff6f5eae676fee712e453c8fb61ed246097d4a0757107`. Captured original compiler arguments and library/header/probe/log hashes, with completed 36-row runs per measured provider.
- `probe` (input): [rust/tcl-vm/tests/data/native_formal_slots/compiled-local-names.c](../../../../rust/tcl-vm/tests/data/native_formal_slots/compiled-local-names.c). SHA-256 `70d3e32511cb1985132f624fbc670402eefda17b6fb18b64a0a09f4c134da684`. Exact counted formal lists, original bodies and completion-first probe.
- `rows-tcl8.4` (observation): [rust/tcl-vm/tests/data/native_formal_slots/compiled-local-names-8.4.20.jsonl](../../../../rust/tcl-vm/tests/data/native_formal_slots/compiled-local-names-8.4.20.jsonl). SHA-256 `32349492892755457a1ed8f4045eb072b94ff4aa698442544fbd23084529037e`. All 36 exact definition/completion records for 8.4.20.
- `rows-tcl8.5` (observation): [rust/tcl-vm/tests/data/native_formal_slots/compiled-local-names-8.5.19.jsonl](../../../../rust/tcl-vm/tests/data/native_formal_slots/compiled-local-names-8.5.19.jsonl). SHA-256 `86dabdaa707558e87df42892b99d197837314c4d6ddbc7174130625afe47d9ca`. All 36 exact definition/completion records for 8.5.19.
- `rows-tcl8.6` (observation): [rust/tcl-vm/tests/data/native_formal_slots/compiled-local-names-8.6.18.jsonl](../../../../rust/tcl-vm/tests/data/native_formal_slots/compiled-local-names-8.6.18.jsonl). SHA-256 `1ef39d66055d854868789bbc5bc5638003ee36a4382cbf4f73766c76c935ab63`. All 36 exact definition/completion records for 8.6.18.
- `rows-tcl9.0` (observation): [rust/tcl-vm/tests/data/native_formal_slots/compiled-local-names-9.0.4.jsonl](../../../../rust/tcl-vm/tests/data/native_formal_slots/compiled-local-names-9.0.4.jsonl). SHA-256 `1ef39d66055d854868789bbc5bc5638003ee36a4382cbf4f73766c76c935ab63`. All 36 exact definition/completion records for 9.0.4.
- `rows-tcl9.1` (observation): [rust/tcl-vm/tests/data/native_formal_slots/compiled-local-names-9.1.0.jsonl](../../../../rust/tcl-vm/tests/data/native_formal_slots/compiled-local-names-9.1.0.jsonl). SHA-256 `f4659fd880cdade2333ed290148126074139963846b92a04fae6cc34355d6549`. All 36 exact definition/completion records for 9.1.0.
- `rows-jim` (observation): [rust/tcl-vm/tests/data/native_formal_slots/compiled-local-names-Jim.jsonl](../../../../rust/tcl-vm/tests/data/native_formal_slots/compiled-local-names-Jim.jsonl). SHA-256 `3194509d8ae3cb1fb396946b0761ec6e3b3929c992532cc19e8f6c60ebea98d7`. All 36 exact definition/completion records for Jim.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-syntax/src/naming/compiled_variables.rs](../../../../rust/tcl-syntax/src/naming/compiled_variables.rs), `NativeCompiledVariableProtocol`: Separate source and runtime compiled-local comparisons; no layout token donor.
- [rust/tcl-syntax/src/naming/compiled_variables.rs](../../../../rust/tcl-syntax/src/naming/compiled_variables.rs), `naming::compiled_variables::tests::compiler_and_dynamic_frame_comparisons_keep_separate_extents` (linked): Static comparison retains equal-count guard while dynamic comparison uses its independently selected extent.
- [rust/tcl-compiler/src/var_escape/original_slots.rs](../../../../rust/tcl-compiler/src/var_escape/original_slots.rs), `var_escape::original_slots::tests::original_argument_primary_collision_matches_retained_native_compiled_results` (linked): Only retained C8.6/C9.0/C9.1 nul-equal-length, two-original-formals and compiled/dynamic invocation rows compare source argument ordinal 1 with the first compiled primary k\x00a; no physical header, layout or current activation claim is supplied.

A named test is a coverage binding, not a claim that it executed.

## Replay

Recorded argument vector:

```json
[
  "cc",
  "-I/path/tcl9.0.4/generic",
  "-I/path/tcl9.0.4/unix",
  "rust/tcl-vm/tests/data/native_formal_slots/compiled-local-names.c",
  "/path/tcl9.0.4/unix/libtcl9.0.a",
  "-lm",
  "-ldl",
  "-lpthread",
  "-lz",
  "-o",
  "/tmp/naming-compiled-local-probe"
]
```

Run the produced probe and compare all 36 JSONL records to the selected retained provider file. Repeat with each manifest-version build; Jim uses -DUSE_JIM and its pinned supplied library. Current build availability and engine revision must be reverified; compile/run success and complete field equality are required. This record does not claim a fresh reconfirmation.
