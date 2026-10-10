// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Physical variable retirement through selected original cell receipts.

use super::Interp;
use crate::{namespace::NsId, vars::TraceHome};

impl Interp {
    /// Retire the selected old registrations before invoking their callbacks.
    /// The supplied home retains root/member identity; it performs no lookup.
    pub(super) fn fire_selected_unset_callbacks(
        &mut self,
        home: &crate::vars::TraceHome,
        root: &[u8],
        element: Option<&[u8]>,
        spelling: &[u8],
        separate: bool,
    ) {
        let mut access = self.trace_access(spelling, root, element, home, true);
        if separate && element.is_some() {
            access.reported = root.to_vec();
        }
        let mut callbacks = Vec::new();
        if access.match_elem.is_some() && access.whole_array {
            let array = self.variable_trace_scope(home, None);
            if !self.active_var_trace_scopes.borrow().contains(&array) {
                callbacks.extend(self.cell_unset_traces(
                    home,
                    None,
                    &access.reported,
                    access.report_elem.as_deref().unwrap_or_default(),
                ));
            }
        }
        callbacks.extend(self.cell_unset_traces(
            home,
            access.match_elem.as_deref(),
            &access.reported,
            access.report_elem.as_deref().unwrap_or_default(),
        ));
        // Retire the old registrations before callbacks can refill the cell.
        // A retained alias can keep its allocation while new registrations arise.
        self.detach_destroyed_trace_group(home, access.match_elem.as_deref());
        self.fire_unset_callbacks(callbacks);
        self.fire_native_error_variable_trace(home, &access, b"unset");
        self.native_precision_trace(home, &access, b"unset");
    }

    pub(super) fn unset_byte_variable_at(
        &mut self,
        spelling: &[u8],
        root: &[u8],
        element: Option<&[u8]>,
        level: usize,
        separate: bool,
    ) -> bool {
        let capture = crate::vars::capture_variable_receiver_at(
            &mut self.frames.borrow_mut(),
            &mut self.namespaces.borrow_mut(),
            root,
            element.map(<[u8]>::to_vec),
            level,
        );
        let (receiver, home) = match capture {
            Ok(Some(capture)) => capture,
            Ok(None) => return false,
            Err(crate::frame::VarError::NameProtocolUnavailable) => {
                self.report_cmd_error(
                    tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "variable removal frame",
                    )
                    .into(),
                );
                return false;
            }
            Err(_) => return false,
        };
        if element.is_none()
            && self.native_invocation_dialect().variable_destruction_protocol(true)
                == Some(tcl_runtime_api::variable_destruction::VariableDestructionProtocol::ArrayLookupThenRootCallbacksThenMembers)
        {
            if let Some(array) = receiver.begin_array_destruction() {
                return self.finish_array_unset(spelling, root, Some(home), array);
            }
        }
        let Ok(removed) = receiver.unset() else {
            return false;
        };
        if self.has_variable_traces() {
            self.fire_selected_unset_callbacks(&home, root, element, spelling, separate);
        }
        removed
    }

    pub(super) fn retire_namespace_variables(&mut self, namespace: NsId) {
        // naming.variable.original-owner-array-trace-retirement-horizon
        // docs/design/analysis/name-resolution-proofs/variable-original-owner-array-trace-retirement-horizon.md
        // Public namespace callbacks and pinned TclDeleteNamespaceVars support
        // this horizon; they do not observe private cell or table identity.
        loop {
            let selected = {
                let mut namespaces = self.namespaces.borrow_mut();
                let table = namespaces.var_table_mut(namespace);
                let Some(name) = table.teardown_names().first().map(|name| name.to_vec()) else {
                    break;
                };
                let receiver = table
                    .capture_receiver(&name, None)
                    .expect("an actual namespace teardown entry has a receiver");
                let home = TraceHome {
                    binding_id: receiver.binding_id(),
                    selected_member: None,
                    ns: Some(namespace),
                    level: None,
                    base: name,
                    link_elem: None,
                };
                (receiver, home)
            };
            let (receiver, home) = selected;
            let base = home.base.clone();
            let binding = home.binding_id;
            let owner = (home.ns, home.level);
            let mut reported = self.namespaces.borrow().qualified_name(namespace);
            if reported != b"::" {
                reported.extend_from_slice(b"::");
            }
            reported.extend_from_slice(&home.base);
            if let Some(array) = receiver.begin_array_destruction() {
                self.finish_array_unset(&reported, &base, Some(home), array);
            } else {
                let callbacks = self.cell_unset_traces(&home, None, &reported, b"");
                self.namespaces
                    .borrow_mut()
                    .var_table_mut(namespace)
                    .remove(&home.base);
                self.detach_destroyed_trace_group(&home, None);
                self.fire_unset_callbacks(callbacks);
            }
            // Namespace teardown discards callback refills and new registrations
            // on this same physical root. Other namespace tokens stay separate.
            self.traces.borrow_mut().traces.retain(|trace| {
                trace.binding_id != binding
                    || !crate::cmd_trace::same_variable(trace, &base, owner.0, owner.1)
            });
            self.invalidate_guard_domain(tcl_runtime_api::guard::GuardDomain::VariableTrace);
            self.namespaces
                .borrow_mut()
                .var_table_mut(namespace)
                .retire_namespace_binding(&base, binding.expect("selected namespace binding"));
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        counters,
        interp::{Code, Interp},
    };

    type OriginalControl = (&'static str, &'static [u8], [&'static str; 6]);

    macro_rules! control {
        ($fixture:literal, $case:literal) => {
            (
                $case,
                include_bytes!(concat!(
                    "../../../../rust/tcl-registry/tests/data/",
                    $fixture,
                    "/",
                    $case,
                    ".tcl"
                ))
                .as_slice(),
                [
                    include_str!(concat!(
                        "../../../../rust/tcl-registry/tests/data/",
                        $fixture,
                        "/8.4.20/",
                        $case,
                        "/stdout"
                    )),
                    include_str!(concat!(
                        "../../../../rust/tcl-registry/tests/data/",
                        $fixture,
                        "/8.5.19/",
                        $case,
                        "/stdout"
                    )),
                    include_str!(concat!(
                        "../../../../rust/tcl-registry/tests/data/",
                        $fixture,
                        "/8.6.18/",
                        $case,
                        "/stdout"
                    )),
                    include_str!(concat!(
                        "../../../../rust/tcl-registry/tests/data/",
                        $fixture,
                        "/9.0.4/",
                        $case,
                        "/stdout"
                    )),
                    include_str!(concat!(
                        "../../../../rust/tcl-registry/tests/data/",
                        $fixture,
                        "/9.1.0/",
                        $case,
                        "/stdout"
                    )),
                    include_str!(concat!(
                        "../../../../rust/tcl-registry/tests/data/",
                        $fixture,
                        "/jim/",
                        $case,
                        "/stdout"
                    )),
                ],
            )
        };
    }

    fn compare_original_controls(controls: &[OriginalControl]) -> usize {
        compare_controls_with(controls, |interp, source| interp.eval_str(source))
    }

    fn compare_controls_with(
        controls: &[OriginalControl],
        evaluate: impl Fn(&mut Interp, &[u8]) -> Code,
    ) -> usize {
        let providers = ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"];
        let mut comparisons = 0;
        for &(case, source, outputs) in controls {
            for (engine, stdout) in providers.iter().zip(outputs) {
                let (expected_code, original) = stdout
                    .lines()
                    .find_map(|line| line.strip_prefix("ORIGINAL|"))
                    .unwrap()
                    .split_once('|')
                    .unwrap();
                let expected: Vec<u8> = original
                    .as_bytes()
                    .chunks_exact(2)
                    .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
                    .collect();
                counters::reset();
                {
                    let mut interp = Interp::with_native_core(
                        crate::interp::default_host(),
                        crate::environment::profile_for_dialect(engine),
                        tcl_registry::special_vars::NativeBootstrapInputs::default(),
                    )
                    .unwrap();
                    crate::cmd_proc::install_stock_scripted_wrappers(&mut interp);
                    let code = evaluate(&mut interp, source);
                    assert!(
                        !interp.host_refusal_pending(),
                        "{engine}/{case}: {:?}",
                        interp.native_access_refusal()
                    );
                    assert_eq!(
                        code.as_int().to_string(),
                        expected_code,
                        "{engine}/{case}: {:?}",
                        interp.result_bytes()
                    );
                    assert_eq!(interp.result_bytes(), expected, "{engine}/{case}");
                }
                assert_eq!(counters::finalize(), 0, "{engine}/{case}");
                assert_eq!(counters::double_free_count(), 0, "{engine}/{case}");
                comparisons += 1;
            }
        }
        comparisons
    }

    #[test]
    fn original_unset_alias_refills_match_six_native_source_results() {
        // naming.variable.original-unset-alias-trace-refill
        // docs/design/analysis/name-resolution-proofs/variable-original-unset-alias-trace-refill.md
        // Whole original script results retain the public refill and fresh trace.
        // Jim's unavailable trace command remains an independent guest outcome.
        const CONTROLS: &[OriginalControl] = &[
            control!("native_variable_trace_refill351", "element-alias-refill"),
            control!("native_variable_trace_refill351", "scalar-alias-refill"),
        ];
        assert_eq!(compare_original_controls(CONTROLS), 12);
    }

    fn compare_byte_unset_refill(controls: &[OriginalControl], separate: bool) -> usize {
        compare_controls_with(controls, |interp, source| {
            let marker = b"unset saved;";
            let starts: Vec<_> = source
                .windows(marker.len())
                .enumerate()
                .filter_map(|(offset, word)| (word == marker).then_some(offset))
                .collect();
            assert_eq!(starts.len(), 1);
            let start = starts[0];
            let code = interp.eval_str(&source[..start]);
            if code != Code::Ok {
                // An unavailable guest trace command never reaches the byte API.
                return code;
            }
            let removed = if separate {
                interp.var_unset_elem(b"a", b"x")
            } else {
                interp.var_unset(b"saved")
            };
            assert!(removed, "selected direct byte unset");
            interp.eval_str(&source[start + marker.len()..])
        })
    }

    #[test]
    fn direct_byte_unset_keeps_callback_refill_registrations() {
        // naming.variable.original-unset-alias-trace-refill
        // docs/design/analysis/name-resolution-proofs/variable-original-unset-alias-trace-refill.md
        // This implementation control replaces one operation between unchanged
        // source prelude/tail. It asserts shared callback disposal chronology;
        // it does not claim a native C API or original operand/header comparison.
        const CONTROLS: &[OriginalControl] = &[
            control!("native_variable_trace_refill351", "element-alias-refill"),
            control!("native_variable_trace_refill351", "scalar-alias-refill"),
        ];
        assert_eq!(compare_byte_unset_refill(CONTROLS, false), 12);
    }

    #[test]
    fn direct_separate_member_unset_keeps_callback_refill_registrations() {
        // naming.variable.original-unset-alias-trace-refill
        // docs/design/analysis/name-resolution-proofs/variable-original-unset-alias-trace-refill.md
        // The byte root/key consumer shares physical disposal. The native proof
        // measures only its original alias script, not this Rust call interface.
        const CONTROLS: &[OriginalControl] = &[control!(
            "native_variable_trace_refill351",
            "element-alias-refill"
        )];
        assert_eq!(compare_byte_unset_refill(CONTROLS, true), 6);
    }

    #[test]
    fn original_array_unset_refills_match_six_native_source_results() {
        // naming.variable.original-array-unset-trace-refill
        // docs/design/analysis/name-resolution-proofs/variable-original-array-unset-trace-refill.md
        // These are ArrayUnset originals, independent of the Unset alias source.
        const CONTROLS: &[OriginalControl] = &[
            control!("native_array_operational_unset355", "array-member-refill"),
            control!("native_array_operational_unset355", "array-root-refill"),
        ];
        assert_eq!(compare_original_controls(CONTROLS), 12);
    }

    #[test]
    fn original_array_unset_keeps_release_selected_member_lookup() {
        // naming.variable.original-array-unset-name-relookup
        // docs/design/analysis/name-resolution-proofs/variable-original-array-unset-name-relookup.md
        // C84/85 relookup each member name; C86+ keep the selected array.
        // All C releases relookup a whole root after its array callback.
        // These public values do not expose private header/table identity.
        const CONTROLS: &[OriginalControl] = &[
            control!("native_array_unset_relookup356", "whole-root-relookup"),
            control!("native_array_unset_relookup356", "member-held-root"),
        ];
        assert_eq!(compare_original_controls(CONTROLS), 12);
    }

    #[test]
    fn direct_varstore_byte_unset_keeps_callback_refill_registrations() {
        // naming.variable.original-unset-alias-trace-refill
        // docs/design/analysis/name-resolution-proofs/variable-original-unset-alias-trace-refill.md
        // This independently tests the Rust operational byte adapter. The native
        // proof measures its original script, not this VarStore call interface.
        const CONTROLS: &[OriginalControl] = &[
            control!("native_variable_trace_refill351", "element-alias-refill"),
            control!("native_variable_trace_refill351", "scalar-alias-refill"),
        ];
        let comparisons = compare_controls_with(CONTROLS, |interp, source| {
            let marker = b"unset saved;";
            let start = source
                .windows(marker.len())
                .position(|word| word == marker)
                .unwrap();
            let code = interp.eval_str(&source[..start]);
            if code != Code::Ok {
                return code;
            }
            let removed = tcl_runtime_api::VarStore::unset_bytes(
                interp,
                tcl_runtime_api::FrameId(0),
                b"saved",
            )
            .unwrap();
            assert!(removed);
            interp.eval_str(&source[start + marker.len()..])
        });
        assert_eq!(comparisons, 12);
    }

    #[test]
    fn direct_noncurrent_varstore_byte_unset_keeps_callback_refill_registrations() {
        // naming.variable.original-unset-alias-trace-refill
        // docs/design/analysis/name-resolution-proofs/variable-original-unset-alias-trace-refill.md
        // A Rust-owned additional frame exercises addressed removal. This is a
        // software control, not a native C API/frame or generated-body proof.
        const CONTROLS: &[OriginalControl] = &[
            control!("native_variable_trace_refill351", "element-alias-refill"),
            control!("native_variable_trace_refill351", "scalar-alias-refill"),
        ];
        let comparisons = compare_controls_with(CONTROLS, |interp, source| {
            let marker = b"unset saved;";
            let start = source
                .windows(marker.len())
                .position(|word| word == marker)
                .unwrap();
            let code = interp.eval_str(&source[..start]);
            if code != Code::Ok {
                return code;
            }
            interp.frames.borrow_mut().push(crate::namespace::GLOBAL);
            let removed = tcl_runtime_api::VarStore::unset_bytes(
                interp,
                tcl_runtime_api::FrameId(0),
                b"saved",
            )
            .unwrap();
            let (_, owner) = interp.frames.borrow_mut().take_frame_for_pop().unwrap();
            let storage = owner.release();
            interp.frames.borrow_mut().recycle_jim_storage(storage);
            assert!(removed);
            interp.eval_str(&source[start + marker.len()..])
        });
        assert_eq!(comparisons, 12);
    }

    #[test]
    fn missing_addressed_frame_cannot_donate_root_variable() {
        // Rust physical frame admission only, not a native Tcl frame observation.
        counters::reset();
        {
            let mut interp = Interp::with_native_core(
                crate::interp::default_host(),
                crate::environment::profile_for_dialect("tcl8.6"),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            assert_eq!(interp.eval_str(b"set v START"), Code::Ok);
            assert!(!interp.var_unset_at(b"::v", 99));
            assert!(interp.host_refusal_pending());
            let value = interp
                .namespaces
                .borrow()
                .var_table(crate::namespace::GLOBAL)
                .load_scalar(b"v")
                .unwrap();
            assert_eq!(crate::interp::obj_bytes(value), b"START");
        }
        assert_eq!(counters::finalize(), 0);
        assert_eq!(counters::double_free_count(), 0);
    }

    #[test]
    fn unretained_array_target_cannot_remove_same_named_member() {
        // An actual allocation ID alone is not the retained operation receipt.
        counters::reset();
        {
            let mut interp = Interp::with_native_core(
                crate::interp::default_host(),
                crate::environment::profile_for_dialect("tcl8.6"),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            assert_eq!(interp.eval_str(b"set a(x) OLD"), Code::Ok);
            let id = crate::vars::array_target_at(
                &interp.frames.borrow(),
                &interp.namespaces.borrow(),
                b"a",
                0,
            )
            .unwrap()
            .id();
            let unretained =
                tcl_runtime_api::ArrayTarget::cell_bytes(tcl_runtime_api::FrameId(0), b"a", id);
            assert!(!interp.array_unset_elem_at_target(&unretained, b"x"));
            assert_eq!(interp.eval_str(b"set a(x)"), Code::Ok);
            assert_eq!(interp.result_bytes(), b"OLD");
            assert!(!interp.host_refusal_pending());
        }
        assert_eq!(counters::finalize(), 0);
        assert_eq!(counters::double_free_count(), 0);
    }

    #[test]
    fn original_undefined_member_callbacks_match_six_native_retirement_sources() {
        // naming.variable.original-undefined-array-member-retirement
        // docs/design/analysis/name-resolution-proofs/variable-original-undefined-array-member-retirement.md
        // Each original result retains array existence/member absence before
        // destruction and the public callback log. No native cell identity claim.
        const CONTROLS: &[OriginalControl] = &[
            control!("native_array_undefined_member350", "explicit-unset"),
            control!("native_array_undefined_member350", "frame-exit"),
            control!("native_array_undefined_member350", "namespace-delete"),
        ];
        assert_eq!(compare_original_controls(CONTROLS), 18);
    }

    #[test]
    fn original_namespace_array_teardown_matches_six_native_owner_sources() {
        // naming.variable.original-owner-array-trace-retirement-horizon
        // docs/design/analysis/name-resolution-proofs/variable-original-owner-array-trace-retirement-horizon.md
        // Three whole namespace results, including Jim's unavailable trace API.
        // The original public logs do not expose private namespace/member identity.
        const CONTROLS: &[OriginalControl] = &[
            control!("native_array_owner_teardown347", "namespace-root"),
            control!("native_array_owner_teardown347", "namespace-recreated"),
            control!("native_array_owner_teardown347", "namespace-foreign"),
        ];
        assert_eq!(compare_original_controls(CONTROLS), 18);
    }

    #[test]
    fn original_frame_alias_refusals_do_not_claim_teardown_chronology() {
        // naming.variable.original-owner-array-trace-retirement-horizon
        // docs/design/analysis/name-resolution-proofs/variable-original-owner-array-trace-retirement-horizon.md
        // naming.variable.original-frame-array-trace-retirement-horizon
        // docs/design/analysis/name-resolution-proofs/variable-original-frame-array-trace-retirement-horizon.md
        // The original C bad-level/inverted-alias errors occur before traces.
        // Jim's unavailable trace command is retained independently.
        const CONTROLS: &[OriginalControl] = &[
            control!("native_array_owner_teardown347", "frame-root"),
            control!("native_array_owner_teardown347", "frame-members"),
            control!("native_array_frame_teardown348", "frame-root"),
            control!("native_array_frame_teardown348", "frame-members"),
        ];
        assert_eq!(compare_original_controls(CONTROLS), 24);
    }

    #[test]
    fn original_frame_callbacks_observe_caller_without_donating_member_traces() {
        // naming.variable.original-frame-array-trace-caller-access
        // docs/design/analysis/name-resolution-proofs/variable-original-frame-array-trace-caller-access.md
        // Positive owner reads and whole callback/caller results are compared.
        // No receipt to an inaccessible departing C frame is fabricated.
        const CONTROLS: &[OriginalControl] = &[
            control!("native_array_frame_access349", "frame-root-access"),
            control!("native_array_frame_access349", "frame-member-access"),
        ];
        assert_eq!(compare_original_controls(CONTROLS), 12);
    }
}
