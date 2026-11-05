# naming.tcloo.variable-info-original-option-boundaries

Kind: `native-observation`

## Problem statement

Counted raw zero, encoded zero and option abbreviations can reach different handler branches before a declaration list exists. Introspection must retain the original option object, consuming extent and selected target/option error order independently of a reporting spelling.

## Question

What code and result bytes do original info object/class variables operands produce for the bounded option, arity and missing-target cases?

## Conclusion

C8.6 accepts the target alone and rejects a second flag by arity. C9.0/C9.1 accept exact -private and its raw-zero suffix, while encoded C080, -priv and bad flags reject before missing-target lookup. Extra arity is checked first. Supported direct counted-object and counted-source controls retain the same measured outcomes. C8.4/C8.5/Jim reject the unavailable info ensemble surface; those rows do not establish OO option semantics.

## Scope

44 fixed controls per provider: 11 operands by object/class and direct counted object argv/count-limited native source, 264 total. Fresh setup/private setup and original option/input/result/return-options rows remain separate. No Document ingress, Index cache, callback, native header/pointer identity, declaration installation or Normal claim; linked Rust tests compare 66 direct C8.6/C9 code/result rows only, with owned declaration rows as explicit query preconditions.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Separate original counted Tcl/Jim string-object argv and counted native source. Raw00 and encoded C080 operands are distinct; no external Document conversion. Version: tcl8.4 8.4.20.

All 44 controls reject unavailable OO info object/class surface; setup errors and missing return-options API are separately retained.

### tcl8.5

Status: `observed`. Channel: Separate original counted Tcl/Jim string-object argv and counted native source. Raw00 and encoded C080 operands are distinct; no external Document conversion. Version: tcl8.5 8.5.19.

All 44 controls reject unavailable OO info object/class surface; exact guest results/options retained.

### tcl8.6

Status: `observed`. Channel: Separate original counted Tcl/Jim string-object argv and counted native source. Raw00 and encoded C080 operands are distinct; no external Document conversion. Version: tcl8.6 8.6.18.

Base public declaration query succeeds; optional flag/extra arity rejects before target, and missing target without flag rejects by object lookup. No private option supported.

### tcl9.0, tcl9.1

Status: `observed`. Channel: Separate original counted Tcl/Jim string-object argv and counted native source. Raw00 and encoded C080 operands are distinct; no external Document conversion. Version: tcl9.0 9.0.4; tcl9.1 9.1.0.

Base/private queries succeed. Exact/raw-zero private flags select; encoded zero, abbreviation and bad flags reject before missing target; extra arity and absent target retain their distinct errors.

### jim

Status: `observed`. Channel: Separate original counted Tcl/Jim string-object argv and counted native source. Raw00 and encoded C080 operands are distinct; no external Document conversion. Version: jim 0.84-9-g5bac7c9.

All 44 controls reject unavailable OO info object/class surface; exact setup/guest result and unqueried C return-options API retained.

### bigip

Status: `not-tested`.

No appliance observation for this question.

## Consumer bindings

- [rust/tcl-registry/src/native_tcloo_info.rs](../../../../rust/tcl-registry/src/native_tcloo_info.rs), `NativeTclooVariableInfoProtocol::private_option`: Selected exact CString option purpose; no target or native object grant.
- [rust/tcl-vm/src/cmd_oo.rs](../../../../rust/tcl-vm/src/cmd_oo.rs), `variable_info_options`: Validate original option/arity before target lookup.
- [runtime/rust/src/cmd_oo.rs](../../../../runtime/rust/src/cmd_oo.rs), `variable_info_options`: Validate original option/arity before target lookup.
- [rust/tcl-vm/src/cmd_oo/native_info_option_tests.rs](../../../../rust/tcl-vm/src/cmd_oo/native_info_option_tests.rs), `cmd_oo::native_info_option_tests::variable_info_matches_original_native_option_and_target_failures` (linked): 66 direct C8.6/C9.0/C9.1 code/result byte comparisons for explicit owned declaration query preconditions; no return-options, original pointer, installer or Rust execution claim.
- [runtime/rust/src/cmd_oo/native_info_option_tests.rs](../../../../runtime/rust/src/cmd_oo/native_info_option_tests.rs), `cmd_oo::native_info_option_tests::variable_info_matches_original_native_option_and_target_failures` (linked): 66 direct C8.6/C9.0/C9.1 code/result byte comparisons for explicit owned declaration query preconditions; no return-options, original pointer, installer or Rust execution claim.

## Evidence

- [rust/tcl-registry/tests/data/native_tcloo_variable_info_original](../../../../rust/tcl-registry/tests/data/native_tcloo_variable_info_original)
- [rust/tcl-registry/tests/data/native_tcloo_variable_info_original/8.4.20](../../../../rust/tcl-registry/tests/data/native_tcloo_variable_info_original/8.4.20)
- [rust/tcl-registry/tests/data/native_tcloo_variable_info_original/8.5.19](../../../../rust/tcl-registry/tests/data/native_tcloo_variable_info_original/8.5.19)
- [rust/tcl-registry/tests/data/native_tcloo_variable_info_original/8.6.18](../../../../rust/tcl-registry/tests/data/native_tcloo_variable_info_original/8.6.18)
- [rust/tcl-registry/tests/data/native_tcloo_variable_info_original/9.0.4](../../../../rust/tcl-registry/tests/data/native_tcloo_variable_info_original/9.0.4)
- [rust/tcl-registry/tests/data/native_tcloo_variable_info_original/9.1.0](../../../../rust/tcl-registry/tests/data/native_tcloo_variable_info_original/9.1.0)
- [rust/tcl-registry/tests/data/native_tcloo_variable_info_original/jim](../../../../rust/tcl-registry/tests/data/native_tcloo_variable_info_original/jim)
