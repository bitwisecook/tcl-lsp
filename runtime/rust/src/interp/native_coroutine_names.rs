// SPDX-License-Identifier: AGPL-3.0-or-later
//! Coroutine publication retains original names and exact existing holders.

use super::Interp;
use crate::{namespace::NsId, obj::TclObj};
use tcl_cmd_core::CmdError;
use tcl_syntax::{
    naming::{NamePolicyAuthority, NativeNameContext},
    value::{ValueError, ValueOps},
};

impl Interp {
    pub(crate) fn native_coroutine_publication_slot(
        &mut self,
        original: *mut TclObj,
    ) -> Result<(NsId, Vec<u8>), CmdError> {
        let bytes = self.native_string_bytes(&original)?;
        let policy = self
            .name_policy_protocol()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "coroutine publication naming issuer",
            ))?;
        let current = self.current_ns();
        let namespaces = self.namespaces();
        let path = namespaces.native_context_path(current).ok_or(
            ValueError::CommandProtocolUnavailable("coroutine publication current holder"),
        )?;
        let selected = policy
            .recipe()
            .coroutine_publication_projection(NativeNameContext::new(&path), &bytes)
            .map_err(|_| {
                ValueError::CommandProtocolUnavailable("coroutine publication name purpose")
            })?;
        let Some(namespace) = namespaces.command_projection_holder(current, &selected) else {
            let diagnostic = policy
                .recipe()
                .coroutine_unknown_namespace_error(&bytes)
                .map_err(|_| {
                    ValueError::CommandProtocolUnavailable("coroutine namespace diagnostic")
                })?;
            let mut error =
                CmdError::with_error_code_bytes(diagnostic.message, diagnostic.error_code);
            if policy.authority() == NamePolicyAuthority::Native {
                error = error.with_native_string_result(policy.string_protocol());
            }
            return Err(error);
        };
        Ok((namespace, selected.slot().simple.as_bytes().to_vec()))
    }

    pub(crate) fn register_coroutine_command_in_slot(&mut self, slot: (NsId, Vec<u8>)) -> u64 {
        self.bind_command_replacement(
            slot.0,
            &slot.1,
            crate::interp::Command::Builtin(crate::cmd_coro::coro_resume_command),
        );
        self.namespaces()
            .command_generation(slot.0, &slot.1)
            .expect("the selected coroutine registration was just published")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::namespace::GLOBAL;
    use crate::obj::{self, Owned};

    fn unhex(value: &str) -> Vec<u8> {
        value
            .as_bytes()
            .chunks_exact(2)
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect()
    }
    fn row<'a>(rows: &'a str, label: &str) -> Vec<&'a str> {
        rows.lines()
            .find(|line| line.split('|').next() == Some(label))
            .unwrap()
            .split('|')
            .collect()
    }

    fn call(interp: &mut Interp, head: &[u8], args: &[*mut TclObj]) -> crate::interp::Code {
        let head = Owned::fresh(obj::new_string_bytes(head));
        let mut words = vec![head.as_ptr()];
        words.extend_from_slice(args);
        interp.dispatch(&words)
    }
    #[test]
    fn original_coroutine_names_match_native_publication_controls() {
        // Native proof: naming.coroutine.original-publication-boundaries
        // docs/design/analysis/name-resolution-proofs/coroutine-original-publication-boundaries.md
        let mut comparisons = 0;
        for (engine, rows) in [
            (
                "tcl8.6",
                include_str!(
                    "../../../../rust/tcl-registry/tests/data/native_coroutine_publication/v2/8.6.18/stdout.tsv"
                ),
            ),
            (
                "tcl9.0",
                include_str!(
                    "../../../../rust/tcl-registry/tests/data/native_coroutine_publication/v2/9.0.4/stdout.tsv"
                ),
            ),
            (
                "tcl9.1",
                include_str!(
                    "../../../../rust/tcl-registry/tests/data/native_coroutine_publication/v2/9.1.0/stdout.tsv"
                ),
            ),
        ] {
            for input in rows.lines().filter(|line| {
                line.starts_with("PUB_") && line.split('|').next().unwrap().ends_with("_INPUT")
            }) {
                let fields: Vec<_> = input.split('|').collect();
                let label = fields[0].strip_suffix("_INPUT").unwrap();
                let mut interp = Interp::with_native_core(
                    crate::interp::default_host(),
                    crate::environment::profile_for_dialect(engine),
                    tcl_registry::special_vars::NativeBootstrapInputs::default(),
                )
                .unwrap();
                for name in [b"::N".as_slice(), b"::N::Q", b"::Q"] {
                    interp.namespaces_mut().ensure_namespace(GLOBAL, name);
                }
                if label.starts_with("PUB_N_") {
                    let current = interp.namespaces().find_namespace(GLOBAL, b"::N").unwrap();
                    interp.current_ns.set(current);
                }
                let name = Owned::fresh(obj::new_string_bytes(&unhex(fields[3])));
                let command = Owned::fresh(obj::new_string_bytes(b"yield"));
                let ready = Owned::fresh(obj::new_string_bytes(b"READY"));
                let code = call(
                    &mut interp,
                    b"coroutine",
                    &[name.as_ptr(), command.as_ptr(), ready.as_ptr()],
                );
                let native = row(rows, label);
                assert_eq!(
                    code.as_int(),
                    native[1].parse::<i64>().unwrap(),
                    "{engine}/{label}"
                );
                assert_eq!(
                    interp.result_bytes(),
                    unhex(native[3]),
                    "{engine}/{label}/bytes"
                );
                assert!(!interp.host_refusal_pending(), "{engine}/{label}/host");
                comparisons += 1;
                if code == crate::interp::Code::Ok {
                    let resumed = Owned::fresh(obj::new_string_bytes(b"RESUMED"));
                    let code = interp.dispatch(&[name.as_ptr(), resumed.as_ptr()]);
                    let native = row(rows, &format!("{label}_RESUME"));
                    assert_eq!(
                        code.as_int(),
                        native[1].parse::<i64>().unwrap(),
                        "{engine}/{label}/resume-code"
                    );
                    assert_eq!(
                        interp.result_bytes(),
                        unhex(native[3]),
                        "{engine}/{label}/resume-bytes"
                    );
                    assert!(
                        !interp.host_refusal_pending(),
                        "{engine}/{label}/resume-host"
                    );
                    comparisons += 1;
                } else {
                    assert_eq!(
                        interp.error_code(),
                        b"TCL LOOKUP NAMESPACE",
                        "{engine}/{label}/error-code"
                    );
                }
                let info = Owned::fresh(obj::new_string_bytes(b"info"));
                let coroutine = Owned::fresh(obj::new_string_bytes(b"coroutine"));
                let code = call(
                    &mut interp,
                    b"coroutine",
                    &[name.as_ptr(), info.as_ptr(), coroutine.as_ptr()],
                );
                let expected = row(rows, &format!("{label}_TOKEN_FULLNAME"));
                assert_eq!(
                    code.as_int(),
                    expected[1].parse::<i64>().unwrap(),
                    "{engine}/{label}/token-fullname-code"
                );
                assert_eq!(
                    interp.result_bytes(),
                    unhex(expected[3]),
                    "{engine}/{label}/token-fullname-bytes"
                );
                assert!(
                    !interp.host_refusal_pending(),
                    "{engine}/{label}/token-fullname-host"
                );
                comparisons += 1;
                assert!(
                    interp
                        .namespaces()
                        .child_token(GLOBAL, b"Missing")
                        .is_none(),
                    "{engine}/{label}/no-missing-namespace-publication"
                );
            }
        }
        assert_eq!(comparisons, 258);
    }
}
