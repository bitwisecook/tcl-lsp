// SPDX-License-Identifier: AGPL-3.0-or-later
//! Coroutine publication retains original names and exact existing holders.

#[cfg(test)]
use super::ROOT_NS;
use super::{CommandSlot, Vm};
use crate::Value;
use tcl_cmd_core::CmdError;
use tcl_syntax::{
    naming::{NamePolicyAuthority, NativeNameContext},
    raw_string::NativeStringAccessError,
    value::ValueError,
};

impl Vm {
    pub(crate) fn native_coroutine_publication_slot(
        &self,
        original: &Value,
    ) -> Result<CommandSlot, CmdError> {
        let bytes = self
            .native_name_operand_bytes(original)
            .map_err(|error| CmdError::from(NativeStringAccessError::Unavailable(error)))?;
        let policy = self
            .name_policy_protocol()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "coroutine publication naming issuer",
            ))?;
        let current = self.current_ns_id();
        let path = self.namespace_path_for_token(current);
        let selected = policy
            .recipe()
            .coroutine_publication_projection(NativeNameContext::new(&path), &bytes)
            .map_err(|_| {
                ValueError::CommandProtocolUnavailable("coroutine publication name purpose")
            })?;
        let Some(namespace) = self.native_command_projection_holder(current, &selected) else {
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
        Ok(CommandSlot {
            namespace,
            simple: selected.into_slot().simple,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_core_types::ByteNamespacePath;

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

    #[test]
    fn original_coroutine_names_match_native_publication_controls() {
        // Native proof: naming.coroutine.original-publication-boundaries
        // docs/design/analysis/name-resolution-proofs/coroutine-original-publication-boundaries.md
        let mut comparisons = 0;
        for (engine, rows) in [
            (
                "tcl8.6",
                include_str!(
                    "../../../tcl-registry/tests/data/native_coroutine_publication/v2/8.6.18/stdout.tsv"
                ),
            ),
            (
                "tcl9.0",
                include_str!(
                    "../../../tcl-registry/tests/data/native_coroutine_publication/v2/9.0.4/stdout.tsv"
                ),
            ),
            (
                "tcl9.1",
                include_str!(
                    "../../../tcl-registry/tests/data/native_coroutine_publication/v2/9.1.0/stdout.tsv"
                ),
            ),
        ] {
            for input in rows.lines().filter(|line| {
                line.starts_with("PUB_") && line.split('|').next().unwrap().ends_with("_INPUT")
            }) {
                let fields: Vec<_> = input.split('|').collect();
                let label = fields[0].strip_suffix("_INPUT").unwrap();
                let mut vm = crate::native_fixture::interpreter(
                    crate::environment::profile_for_dialect(engine),
                );
                for path in [
                    ByteNamespacePath::from_segments([b"N".as_slice()]),
                    ByteNamespacePath::from_segments([b"N".as_slice(), b"Q"]),
                    ByteNamespacePath::from_segments([b"Q".as_slice()]),
                ] {
                    vm.declare_namespace_path_with_origin(path, true);
                }
                if label.starts_with("PUB_N_") {
                    let current = vm
                        .native_namespace_context_for_path(&ByteNamespacePath::from_segments([
                            b"N".as_slice(),
                        ]))
                        .unwrap();
                    vm.resolution_stacks.ns_id_stack.push(current);
                }
                let name = Value::new_native_string_bytes(unhex(fields[3]));
                let completion = vm.invoke_host_original_object_vector(
                    &Value::new_native_string_bytes(b"coroutine".as_slice()),
                    &[
                        name.clone(),
                        Value::new_native_string_bytes(b"yield".as_slice()),
                        Value::new_native_string_bytes(b"READY".as_slice()),
                    ],
                );
                let native = row(rows, label);
                assert_eq!(
                    completion.code.as_int(),
                    native[1].parse::<i64>().unwrap(),
                    "{engine}/{label}"
                );
                let protocol = vm
                    .actual_native_invocation_dialect()
                    .native_string_protocol()
                    .unwrap();
                assert_eq!(
                    completion
                        .result
                        .native_string_bytes(protocol)
                        .unwrap()
                        .as_ref(),
                    unhex(native[3]),
                    "{engine}/{label}/bytes"
                );
                comparisons += 1;
                if completion.code == tcl_core_types::Code::Ok {
                    let resumed = vm.invoke_host_original_object_vector(
                        &name,
                        &[Value::new_native_string_bytes(b"RESUMED".as_slice())],
                    );
                    let native = row(rows, &format!("{label}_RESUME"));
                    assert_eq!(
                        resumed.code.as_int(),
                        native[1].parse::<i64>().unwrap(),
                        "{engine}/{label}/resume-code"
                    );
                    assert_eq!(
                        resumed
                            .result
                            .native_string_bytes(protocol)
                            .unwrap()
                            .as_ref(),
                        unhex(native[3]),
                        "{engine}/{label}/resume-bytes"
                    );
                    comparisons += 1;
                } else {
                    assert_eq!(
                        crate::command::opt_get(&completion.options, "-errorcode")
                            .unwrap()
                            .native_string_bytes(protocol)
                            .unwrap()
                            .as_ref(),
                        b"TCL LOOKUP NAMESPACE",
                        "{engine}/{label}/error-code"
                    );
                }
                let fullname = vm.invoke_host_original_object_vector(
                    &Value::new_native_string_bytes(b"coroutine".as_slice()),
                    &[
                        name.clone(),
                        Value::new_native_string_bytes(b"info".as_slice()),
                        Value::new_native_string_bytes(b"coroutine".as_slice()),
                    ],
                );
                let expected = row(rows, &format!("{label}_TOKEN_FULLNAME"));
                assert_eq!(
                    fullname.code.as_int(),
                    expected[1].parse::<i64>().unwrap(),
                    "{engine}/{label}/token-fullname-code"
                );
                assert_eq!(
                    fullname
                        .result
                        .native_string_bytes(protocol)
                        .unwrap()
                        .as_ref(),
                    unhex(expected[3]),
                    "{engine}/{label}/token-fullname-bytes"
                );
                comparisons += 1;
                assert!(
                    vm.namespace_child_token(ROOT_NS, b"Missing").is_none(),
                    "{engine}/{label}/no-missing-namespace-publication"
                );
            }
        }
        assert_eq!(comparisons, 258);
    }
}
