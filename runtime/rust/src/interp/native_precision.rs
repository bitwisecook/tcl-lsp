// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Hidden precision callbacks follow the selected physical variable cell.

use super::{Interp, TraceAccess};
use crate::{frame::VariableReceiver, obj, vars::TraceHome};

impl Interp {
    pub(super) fn install_native_precision_trace(&mut self) {
        self.native_precision_cell.set(None);
        if let Some(name) = self
            .native_invocation_dialect()
            .double_string_policy()
            .and_then(tcl_dialect::DoubleStringPolicy::precision_variable)
        {
            self.install_native_precision_trace_at(name.as_bytes());
        }
    }

    pub(super) fn install_native_precision_trace_at(&mut self, name: &[u8]) {
        self.native_precision_cell.set(None);
        if self.pending_delete.get() || self.require_variable_name_protocol().is_err() {
            return;
        }
        // TclPrecTraceProc reinstalls with GLOBAL_ONLY and the original
        // callback name. A proc-local upvar spelling can relocate this trace.
        let home = crate::vars::ensure_undefined_at(
            &mut self.frames.borrow_mut(),
            &mut self.namespaces.borrow_mut(),
            name,
            0,
        );
        self.native_precision_cell
            .set(home.ok().and_then(|home| home.binding_id));
    }

    pub(super) fn native_precision_trace_at(&self, home: &TraceHome) -> bool {
        self.native_invocation_dialect()
            .double_string_policy()
            .is_some_and(tcl_dialect::DoubleStringPolicy::has_precision_variable)
            && home.binding_id.is_some()
            && home.binding_id == self.native_precision_cell.get()
    }

    fn native_precision_receiver(
        &self,
        home: &TraceHome,
        element: Option<&[u8]>,
    ) -> Option<VariableReceiver> {
        let (receiver, _) = crate::vars::capture_original_namespace_receiver(
            &mut self.frames.borrow_mut(),
            &mut self.namespaces.borrow_mut(),
            home.ns?,
            &home.base,
            element.map(<[u8]>::to_vec),
            false,
        )
        .ok()??;
        (receiver.binding_id() == home.binding_id).then_some(receiver)
    }

    pub(super) fn native_precision_trace(
        &mut self,
        home: &TraceHome,
        access: &TraceAccess,
        op: &[u8],
    ) -> bool {
        if !self.native_precision_trace_at(home) {
            return false;
        }
        if op == b"unset" {
            if access.match_elem.is_none() {
                self.install_native_precision_trace_at(&access.reported);
            }
            return false;
        }
        if op == b"array" {
            return false;
        }
        // The intrinsic belongs to the containing array. C8.x element aliases
        // suppress that array's callbacks, including the hidden callback.
        if access.match_elem.is_some() {
            let cell = self.variable_trace_scope(home, access.match_elem.as_deref());
            if !access.whole_array
                || self
                    .active_var_trace_scopes
                    .borrow()
                    .contains(&cell.array())
            {
                return false;
            }
        }
        let Some(receiver) = self.native_precision_receiver(home, access.match_elem.as_deref())
        else {
            return false;
        };
        let policy = self
            .native_invocation_dialect()
            .double_string_policy()
            .expect("selected hidden precision callback");
        if op == b"read" {
            restore_precision(&receiver, policy);
            return false;
        }
        self.write_native_precision(&receiver, policy)
    }

    fn write_native_precision(
        &mut self,
        receiver: &VariableReceiver,
        policy: tcl_dialect::DoubleStringPolicy,
    ) -> bool {
        let value = receiver.read().ok().flatten().map(super::obj_bytes);
        let precision = value
            .as_ref()
            .and_then(|value| std::str::from_utf8(value).ok())
            .and_then(|value| tcl_syntax::number::parse_double_precision(policy, value));
        let error = if self.is_safe() {
            Some(b"can't modify precision from a safe interpreter".as_slice())
        } else if let Some(precision) = precision {
            obj::set_double_precision(policy, precision);
            None
        } else {
            Some(b"improper value for precision".as_slice())
        };
        if let Some(error) = error {
            if policy == tcl_dialect::DoubleStringPolicy::Tcl84Precision {
                restore_precision(receiver, policy);
            }
            self.traces.borrow_mut().pending_err = Some(error.to_vec());
            return true;
        }
        false
    }
}

fn restore_precision(receiver: &VariableReceiver, policy: tcl_dialect::DoubleStringPolicy) {
    let value = obj::Owned::fresh(obj::new_wide_int_obj(i64::from(obj::double_precision(
        policy,
    ))));
    let _ = receiver.store(value.as_ptr());
}

#[cfg(test)]
mod tests {
    use super::Interp;

    #[test]
    fn original_precision_unset_matches_all_six_native_sources() {
        // naming.variable.original-precision-unset-publication
        // docs/design/analysis/name-resolution-proofs/variable-original-precision-unset-publication.md
        // Exact public results only; these rows do not observe hidden cell
        // identities or establish a pointer-equivalence claim.
        macro_rules! provider {
            ($engine:literal, $version:literal) => {
                (
                    $engine,
                    [
                        include_str!(concat!(
                            "../../../../rust/tcl-registry/tests/data/native_precision_unset319/",
                            $version,
                            "/root/stdout"
                        )),
                        include_str!(concat!(
                            "../../../../rust/tcl-registry/tests/data/native_precision_unset319/",
                            $version,
                            "/linked/stdout"
                        )),
                        include_str!(concat!(
                            "../../../../rust/tcl-registry/tests/data/native_precision_unset319/",
                            $version,
                            "/unrelated/stdout"
                        )),
                    ],
                )
            };
        }
        let sources = [
            include_str!(
                "../../../../rust/tcl-registry/tests/data/native_precision_unset319/root.tcl"
            ),
            include_str!(
                "../../../../rust/tcl-registry/tests/data/native_precision_unset319/linked.tcl"
            ),
            include_str!(
                "../../../../rust/tcl-registry/tests/data/native_precision_unset319/unrelated.tcl"
            ),
        ];
        for (engine, outputs) in [
            provider!("tcl8.4", "8.4.20"),
            provider!("tcl8.5", "8.5.19"),
            provider!("tcl8.6", "8.6.18"),
            provider!("tcl9.0", "9.0.4"),
            provider!("tcl9.1", "9.1.0"),
            provider!("jim", "jim"),
        ] {
            for (source, stdout) in sources.iter().zip(outputs) {
                let original = stdout
                    .lines()
                    .find_map(|line| line.strip_prefix("ORIGINAL|0|"))
                    .unwrap();
                let expected: Vec<u8> = original
                    .as_bytes()
                    .chunks_exact(2)
                    .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
                    .collect();
                crate::counters::reset();
                {
                    let mut interp = Interp::with_native_core(
                        super::super::default_host(),
                        crate::environment::profile_for_dialect(engine),
                        tcl_registry::special_vars::NativeBootstrapInputs::default(),
                    )
                    .unwrap();
                    assert_eq!(
                        interp.eval_str(source.as_bytes()),
                        super::super::Code::Ok,
                        "{engine}: {:?}, admission {:?}, access {:?}",
                        interp.result_bytes(),
                        interp.native_compilation_admission_error(),
                        interp.native_access_refusal()
                    );
                    assert!(!interp.host_refusal_pending(), "{engine}");
                    assert_eq!(interp.result_bytes(), expected, "{engine}");
                }
                assert_eq!(crate::counters::finalize(), 0, "{engine}");
                assert_eq!(crate::counters::double_free_count(), 0, "{engine}");
            }
        }
    }
}
