# naming.variable.original-array-read-destruction

Kind: `native-observation`

## Problem statement

An alias read during root array-unset and an already-active read callback are different public controls. A missing trace API or equal value alone cannot establish member lifetime/observer guard internals.

## Question

During whole-array destruction, what retained alias value and trace-order results are observed for an inactive versus already-active member read callback?

## Conclusion

All five C providers report `X {ROOTUNSET ELEMENTREAD}` for the root-unset alias-read control, and `1 X {ELEMENTREAD ROOTUNSET}` for the already-active member-read control. Jim reports ORIGINAL code1 with invalid command name "trace" in both untouched setups, so no read/unset callback is observed there. All12 original source processes and six compiler commands exit0 with empty stderr. The original public value/log sequence distinguishes the finite controls but does not observe private Var allocation, trace flags/member IDs, pointer lifetimes or general callback purity. The linked Runtime control compares both untouched original read/destruction sources against every one of the six provider columns, retaining Jim trace setup failure separately from entered C callbacks. The software TraceHome/selected-member owner independently captures exact member IDs within a detached generation and selects current same-key members through the actual receiver; that Runtime correspondence does not turn public value/log equality into native private member, guard or pointer identity. The linked software undefined-scalar control retains the actual captured root binding in TraceHome even while no set namespace cell exists. An unrelated captured root or a replacement after unset cannot reuse that home; member selection stays absent for the scalar. This authored root-ID correspondence and refusal are independent of the native read/unset value/log rows and observe no native private binding/member pointer or active guard.

## Scope

Two exact existing ASCII LF command-binding vector sources run separately in fresh full provider processes through the unchanged counted-source driver. They retain complete original completion code/result rather than reconstructed trace rows. Jim unsupported trace setup is an observed guest failure, distinct from process failure and C callback results. BIG-IP and Rust assertions are not tested.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: External C API driver: fresh full Tcl_Init or Jim core/static extensions per control; independent exact prelude, original counted source, query evaluations in one interpreter; source is not surrounded by catch/eval/puts; C Tcl_EvalEx flags0, Jim original counted object Jim_EvalObj; public counted hex completions only. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

Exact original completion fields: variable_array_unset_live_member_read_trace_fires=0/X {ROOTUNSET ELEMENTREAD}; variable_array_unset_active_member_read_guard_survives=0/1 X {ELEMENTREAD ROOTUNSET}. No private lifetime/trace-guard claim follows from public values/logs.

### jim

Status: `observed`. Channel: External C API driver: fresh full Tcl_Init or Jim core/static extensions per control; independent exact prelude, original counted source, query evaluations in one interpreter; source is not surrounded by catch/eval/puts; C Tcl_EvalEx flags0, Jim original counted object Jim_EvalObj; public counted hex completions only. Version: jim 0.84-9-g5bac7c9.

Exact original completion fields: variable_array_unset_live_member_read_trace_fires=1/invalid command name "trace"; variable_array_unset_active_member_read_guard_survives=1/invalid command name "trace". No private lifetime/trace-guard claim follows from public values/logs.

### bigip

Status: `not-tested`.

No appliance execution answers this exact question.

## Consumer bindings

- [runtime/rust/src/cmd_var.rs](../../../../runtime/rust/src/cmd_var.rs), `cmd_var::tests::original_array_read_destruction_matches_six_native_source_results` (linked): Compare all12 complete original code/result observations with exact unchanged source; C read/unset logs and Jim unavailable trace setup stay separate. Selected Runtime receiver/member ownership is not a Native private guard/lifetime observation.
- [runtime/rust/src/vars.rs](../../../../runtime/rust/src/vars.rs), `vars::tests::selected_scalar_trace_home_retains_the_actual_undefined_root_binding` (linked): Actual captured undefined scalar software binding retains TraceHome correspondence despite absent set cell; unrelated root and unset/replacement refuse, scalar has no selected member. Native public source324 rows do not observe these Rust IDs or prove physical native lifetime.

## Evidence

- [rust/tcl-registry/tests/data](../../../../rust/tcl-registry/tests/data)
- [rust/tcl-registry/tests/data/native_array_read_destruction324/8.4.20](../../../../rust/tcl-registry/tests/data/native_array_read_destruction324/8.4.20)
- [rust/tcl-registry/tests/data/native_array_read_destruction324/8.5.19](../../../../rust/tcl-registry/tests/data/native_array_read_destruction324/8.5.19)
- [rust/tcl-registry/tests/data/native_array_read_destruction324/8.6.18](../../../../rust/tcl-registry/tests/data/native_array_read_destruction324/8.6.18)
- [rust/tcl-registry/tests/data/native_array_read_destruction324/9.0.4](../../../../rust/tcl-registry/tests/data/native_array_read_destruction324/9.0.4)
- [rust/tcl-registry/tests/data/native_array_read_destruction324/9.1.0](../../../../rust/tcl-registry/tests/data/native_array_read_destruction324/9.1.0)
- [rust/tcl-registry/tests](../../../../rust/tcl-registry/tests)
- [rust/tcl-registry/tests/data/native_array_read_destruction324/jim](../../../../rust/tcl-registry/tests/data/native_array_read_destruction324/jim)
