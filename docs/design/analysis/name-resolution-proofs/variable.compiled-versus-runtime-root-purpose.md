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

### tcl8.4, tcl8.5

Status: `observed`. Channel: original counted objects and procedure body via native API. Version: tcl8.4 8.4.20; tcl8.5 8.5.19.

In raw-NUL equal-count one-formal control both compiled reads fail, while both dynamic reads find ONE; two formals fail arity after old list extent. Full case answers are reproduced below.

### tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: original counted objects and procedure body via native API. Version: tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

Equal-count raw-NUL names share first compiled read ONE; dynamic second name is missing with one formal and TWO with two. Full case answers are reproduced below.

### jim

Status: `observed`. Channel: original counted objects and procedure body via native API. Version: jim Jim.

Equal-count raw-NUL names remain distinct in both compiled-source and dynamic lookup: ONE/missing with one formal, ONE/TWO with two. Full case answers are reproduced below.

### bigip

Status: `not-tested`.

No appliance observation for this question.

## Consumer bindings

- [rust/tcl-syntax/src/naming/compiled_variables.rs](../../../../rust/tcl-syntax/src/naming/compiled_variables.rs), `NativeCompiledVariableProtocol`: Separate source and runtime compiled-local comparisons; no layout token donor.
- [rust/tcl-syntax/src/naming/compiled_variables.rs](../../../../rust/tcl-syntax/src/naming/compiled_variables.rs), `naming::compiled_variables::tests::compiler_and_dynamic_frame_comparisons_keep_separate_extents` (linked): Static comparison retains equal-count guard while dynamic comparison uses its independently selected extent.
- [rust/tcl-compiler/src/var_escape/original_slots.rs](../../../../rust/tcl-compiler/src/var_escape/original_slots.rs), `var_escape::original_slots::tests::original_argument_primary_collision_matches_retained_native_compiled_results` (linked): Only retained C8.6/C9.0/C9.1 nul-equal-length, two-original-formals and compiled/dynamic invocation rows compare source argument ordinal 1 with the first compiled primary k\x00a; no physical header, layout or current activation claim is supplied.

## Evidence

- [rust/tcl-vm/tests/data/native_formal_slots/compiled-local-names-manifest.json](../../../../rust/tcl-vm/tests/data/native_formal_slots/compiled-local-names-manifest.json)
- [rust/tcl-vm/tests/data/native_formal_slots/compiled-local-names.c](../../../../rust/tcl-vm/tests/data/native_formal_slots/compiled-local-names.c)
- [rust/tcl-vm/tests/data/native_formal_slots/compiled-local-names-8.6.18.jsonl](../../../../rust/tcl-vm/tests/data/native_formal_slots/compiled-local-names-8.6.18.jsonl)
- [rust/tcl-vm/tests/data/native_formal_slots/compiled-local-names-9.0.4.jsonl](../../../../rust/tcl-vm/tests/data/native_formal_slots/compiled-local-names-9.0.4.jsonl)
- [rust/tcl-vm/tests/data/native_formal_slots/compiled-local-names-9.1.0.jsonl](../../../../rust/tcl-vm/tests/data/native_formal_slots/compiled-local-names-9.1.0.jsonl)
