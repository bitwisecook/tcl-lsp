// SPDX-License-Identifier: AGPL-3.0-or-later
//! Declaring-provider variable resolution in the actual selected method frame.

use super::{DeclaredVariable, NsId, Vm, private_storage_name_bytes};
use tcl_core_types::NameBytes;
use tcl_syntax::naming::{
    NameProjectionUnavailable, NativeNameProtocol, NativeOoVariableResolverPurpose,
    native_oo_variable_resolver_matches,
};

type ProviderDeclarations<'a> = (NsId, u64, &'a [DeclaredVariable], &'a [DeclaredVariable]);

fn declarations(
    vm: &Vm,
    start: usize,
) -> Result<Option<ProviderDeclarations<'_>>, NameProjectionUnavailable> {
    let (activation, frame_namespace) = vm
        .native_oo_frame_identity(start)
        .ok_or(NameProjectionUnavailable::PurposeNotModelled)?;
    let Some(context) = vm
        .oo
        .call_stack
        .iter()
        .rev()
        .find(|context| context.activation == Some(activation))
    else {
        return Ok(None);
    };
    let object = vm
        .oo
        .objects
        .get(&context.object)
        .ok_or(NameProjectionUnavailable::PurposeNotModelled)?;
    let namespace = object
        .namespace_token
        .ok_or(NameProjectionUnavailable::PurposeNotModelled)?;
    if namespace != frame_namespace {
        return Ok(None);
    }
    let step = context
        .chain
        .get(context.index)
        .ok_or(NameProjectionUnavailable::PurposeNotModelled)?;
    let epoch = vm
        .oo
        .objects
        .get(&step.provider)
        .ok_or(NameProjectionUnavailable::PurposeNotModelled)?
        .creation_id;
    let (public, private) = if step.is_object {
        let owner = vm
            .oo
            .objects
            .get(&step.provider)
            .ok_or(NameProjectionUnavailable::PurposeNotModelled)?;
        (&owner.variables, &owner.private_variables)
    } else {
        let owner = vm
            .oo
            .classes
            .get(&step.provider)
            .ok_or(NameProjectionUnavailable::PurposeNotModelled)?;
        (&owner.variables, &owner.private_variables)
    };
    Ok(Some((namespace, epoch, public, private)))
}

/// The runtime resolver gets a `CString` root; declaration keys stay counted.
/// This selects a namespace binding, without inventing a local alias or cell.
pub(crate) fn runtime_target(
    vm: &Vm,
    start: usize,
    protocol: NativeNameProtocol,
    root: &[u8],
) -> Result<Option<(NsId, NameBytes)>, NameProjectionUnavailable> {
    let Some((namespace, epoch, public, private)) = declarations(vm, start)? else {
        return Ok(None);
    };
    for declaration in private {
        if native_oo_variable_resolver_matches(
            protocol,
            NativeOoVariableResolverPurpose::RuntimeRoot,
            declaration.name.as_bytes(),
            root,
        )? {
            return Ok(Some((
                namespace,
                private_storage_name_bytes(epoch, declaration.name.as_bytes()),
            )));
        }
    }
    for declaration in public {
        if native_oo_variable_resolver_matches(
            protocol,
            NativeOoVariableResolverPurpose::RuntimeRoot,
            declaration.name.as_bytes(),
            root,
        )? {
            return Ok(Some((namespace, declaration.name.clone())));
        }
    }
    Ok(None)
}

/// Native `AppendLocals` reports declaring names independently of installed aliases.
pub(crate) fn reported_bindings(
    vm: &Vm,
    start: usize,
) -> Result<Vec<(NsId, NameBytes, NameBytes)>, NameProjectionUnavailable> {
    let Some((namespace, epoch, public, private)) = declarations(vm, start)? else {
        return Ok(Vec::new());
    };
    Ok(public
        .iter()
        .map(|entry| (namespace, entry.name.clone(), entry.name.clone()))
        .chain(private.iter().map(|entry| {
            (
                namespace,
                entry.name.clone(),
                private_storage_name_bytes(epoch, entry.name.as_bytes()),
            )
        }))
        .collect())
}

pub(crate) fn reported_names(
    vm: &Vm,
    start: usize,
) -> Result<Vec<NameBytes>, NameProjectionUnavailable> {
    reported_bindings(vm, start)
        .map(|bindings| bindings.into_iter().map(|(_, name, _)| name).collect())
}

#[cfg(test)]
mod tests {
    use crate::{Vm, value::Value};
    use tcl_runtime_api::{Code, Completion};
    const CASES: &[(&str, &[u8], &[u8])] = &[
        ("PLAIN", b"k", b"k"),
        ("RAW_ZERO", b"k\0tail", b"k\0tail"),
        ("QUALIFIED_AFTER_ZERO", b"k\0::tail", b"k\0::tail"),
        ("SURROGATE", b"k\xed\xa0\x80", b"k\xed\xa0\x80"),
        ("OPAQUE_FF", b"k\xff", b"k\xff"),
        ("PREFIX_DECLARATION_NEGATIVE", b"k", b"k\0tail"),
    ];

    fn call(vm: &mut Vm, words: &[&[u8]]) -> Completion<Value> {
        let arguments: Vec<_> = words
            .iter()
            .map(|word| Value::new_native_string_bytes(*word))
            .collect();
        vm.invoke_host_original_object_vector(&arguments[0], &arguments[1..])
    }

    fn assert_row(vm: &Vm, result: &Completion<Value>, rows: &str, label: &str) {
        let fields: Vec<_> = rows
            .lines()
            .find(|row| row.starts_with(&format!("{label}|")))
            .unwrap()
            .split('|')
            .collect();
        let expected: Vec<_> = fields[2]
            .as_bytes()
            .chunks_exact(2)
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect();
        assert_eq!(
            result.code.as_int(),
            fields[1].parse::<i64>().unwrap(),
            "{label}: {result:?}"
        );
        let protocol = vm
            .actual_native_invocation_dialect()
            .native_string_protocol()
            .unwrap();
        let actual = result.result.native_string_bytes(protocol).unwrap();
        let observed = if label.ends_with("_OBJECT_VARS_AFTER_STATIC")
            || label.ends_with("_OBJECT_VARS_AFTER_DYNAMIC")
        {
            protocol.legacy_eval_result_input(&actual).unwrap()
        } else {
            &actual
        };
        assert_eq!(observed, expected, "{label}");
    }

    #[test]
    fn compiled_and_dynamic_declared_variables_match_original_native_controls() {
        // Native proof: naming.interpreter.legacy-eval-object-result-boundary
        // docs/design/analysis/name-resolution-proofs/legacy-eval-object-result-boundary.md
        // Native proof: naming.tcloo.resident-variable-public-api-counted-keys
        // docs/design/analysis/name-resolution-proofs/tcloo-resident-variable-public-api-counted-keys.md
        // Native proof: naming.tcloo.declared-variable-compiled-and-runtime-resolver
        // docs/design/analysis/name-resolution-proofs/declared-variable-compiled-and-runtime-resolver.md
        for (engine, rows, counted_rows) in [
            (
                "tcl8.6",
                include_str!("../../tests/data/native_oo_variable_resolver/8.6.18/stdout.tsv"),
                include_str!("../../tests/data/native_oo_resident_keys/v5/8.6.18/stdout.tsv"),
            ),
            (
                "tcl9.0",
                include_str!("../../tests/data/native_oo_variable_resolver/9.0.4/stdout.tsv"),
                include_str!("../../tests/data/native_oo_resident_keys/v5/9.0.4/stdout.tsv"),
            ),
            (
                "tcl9.1",
                include_str!("../../tests/data/native_oo_variable_resolver/9.1.0/stdout.tsv"),
                include_str!("../../tests/data/native_oo_resident_keys/v5/9.1.0/stdout.tsv"),
            ),
        ] {
            for &(label, declaration, primary) in CASES {
                let mut vm = crate::native_fixture::interpreter(
                    crate::environment::profile_for_dialect(engine),
                );
                assert_eq!(
                    vm.try_eval_source_bytes(b"oo::class create C {};C create c")
                        .unwrap()
                        .code,
                    Code::Ok
                );
                let declared = call(&mut vm, &[b"oo::define", b"C", b"variable", declaration]);
                assert_row(&vm, &declared, rows, &format!("{label}_DECLARE"));
                let writer = [
                    b"set {".as_slice(),
                    primary,
                    b"} STATIC;return [list [info locals] [info vars]]",
                ]
                .concat();
                let reader = [b"return ${".as_slice(), primary, b"}"].concat();
                for (name, parameters, body) in [
                    (b"writer".as_slice(), b"".as_slice(), writer.as_slice()),
                    (b"reader", b"", reader.as_slice()),
                    (
                        b"dynamic",
                        b"target",
                        b"set $target DYNAMIC;return [list [info locals] [info vars]]".as_slice(),
                    ),
                    (b"dynamicread", b"target", b"set $target".as_slice()),
                ] {
                    assert_eq!(
                        call(
                            &mut vm,
                            &[b"oo::define", b"C", b"method", name, parameters, body]
                        )
                        .code,
                        Code::Ok
                    );
                }
                for (suffix, words) in [
                    ("STATIC_WRITE", vec![b"c".as_slice(), b"writer"]),
                    ("STATIC_READ", vec![b"c".as_slice(), b"reader"]),
                    (
                        "OBJECT_VARS_AFTER_STATIC",
                        vec![b"info".as_slice(), b"object", b"vars", b"c"],
                    ),
                    ("DYNAMIC_WRITE", vec![b"c".as_slice(), b"dynamic", primary]),
                    (
                        "STATIC_READ_AFTER_DYNAMIC",
                        vec![b"c".as_slice(), b"reader"],
                    ),
                    (
                        "DYNAMIC_READ",
                        vec![b"c".as_slice(), b"dynamicread", primary],
                    ),
                    (
                        "OBJECT_VARS_AFTER_DYNAMIC",
                        vec![b"info".as_slice(), b"object", b"vars", b"c"],
                    ),
                ] {
                    let result = call(&mut vm, &words);
                    assert_row(&vm, &result, rows, &format!("{label}_{suffix}"));
                    if suffix == "OBJECT_VARS_AFTER_STATIC" {
                        assert_row(
                            &vm,
                            &result,
                            counted_rows,
                            &format!("{label}_OBJECT_VARS_AFTER_STATIC_OBJECT_VECTOR"),
                        );
                    }
                }
            }
        }
    }
}
