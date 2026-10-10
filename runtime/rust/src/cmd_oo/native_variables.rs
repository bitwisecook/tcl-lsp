// SPDX-License-Identifier: AGPL-3.0-or-later
//! Current declaring-provider lookup, retained by one actual method activation.

use super::{NsId, OoId, OoState};
use std::cell::RefCell;
use std::rc::{Rc, Weak};
use tcl_syntax::naming::{
    native_oo_variable_resolver_matches, NameProjectionUnavailable, NativeNameProtocol,
    NativeOoVariableResolverPurpose,
};

/// Namespace storage, declared source name and physical storage key.
pub(crate) type DeclaredVariableBinding = (NsId, Vec<u8>, Vec<u8>);

pub(crate) struct NativeOoVariableResolver {
    state: Weak<RefCell<OoState>>,
    provider: OoId,
    is_object: bool,
    namespace: NsId,
}

impl NativeOoVariableResolver {
    /// Issued only by the reached method body, before entering its real frame.
    pub(super) fn new(
        state: &Rc<RefCell<OoState>>,
        provider: OoId,
        is_object: bool,
        namespace: NsId,
    ) -> Self {
        Self {
            state: Rc::downgrade(state),
            provider,
            is_object,
            namespace,
        }
    }

    fn declarations<T>(
        &self,
        read: impl FnOnce(u64, &[super::DeclaredVariable], &[super::DeclaredVariable]) -> T,
    ) -> Result<T, NameProjectionUnavailable> {
        let state = self
            .state
            .upgrade()
            .ok_or(NameProjectionUnavailable::PurposeNotModelled)?;
        let state = state.borrow();
        let epoch = state
            .objects
            .get(&self.provider)
            .ok_or(NameProjectionUnavailable::PurposeNotModelled)?
            .creation_id;
        let (public, private) = if self.is_object {
            let owner = state
                .objects
                .get(&self.provider)
                .ok_or(NameProjectionUnavailable::PurposeNotModelled)?;
            (&owner.variables, &owner.private_variables)
        } else {
            let owner = state
                .classes
                .get(&self.provider)
                .ok_or(NameProjectionUnavailable::PurposeNotModelled)?;
            (&owner.variables, &owner.private_variables)
        };
        Ok(read(epoch, public, private))
    }

    pub(crate) fn namespace(&self) -> NsId {
        self.namespace
    }

    /// A runtime root is independently selected; no local alias is manufactured.
    pub(crate) fn runtime_target(
        &self,
        protocol: NativeNameProtocol,
        supplied: &[u8],
    ) -> Result<Option<(NsId, Vec<u8>)>, NameProjectionUnavailable> {
        self.declarations(|epoch, public, private| {
            for declaration in private {
                if native_oo_variable_resolver_matches(
                    protocol,
                    NativeOoVariableResolverPurpose::RuntimeRoot,
                    &declaration.name,
                    supplied,
                )? {
                    let mut name = format!("{epoch} : ").into_bytes();
                    name.extend_from_slice(&declaration.name);
                    return Ok(Some((self.namespace, name)));
                }
            }
            for declaration in public {
                if native_oo_variable_resolver_matches(
                    protocol,
                    NativeOoVariableResolverPurpose::RuntimeRoot,
                    &declaration.name,
                    supplied,
                )? {
                    return Ok(Some((self.namespace, declaration.name.clone())));
                }
            }
            Ok(None)
        })?
    }

    /// Declaring names and namespace storage stay independent of local aliases.
    pub(crate) fn reported_bindings(
        &self,
    ) -> Result<Vec<DeclaredVariableBinding>, NameProjectionUnavailable> {
        self.declarations(|epoch, public, private| {
            public
                .iter()
                .map(|entry| (self.namespace, entry.name.clone(), entry.name.clone()))
                .chain(private.iter().map(|entry| {
                    let mut storage = format!("{epoch} : ").into_bytes();
                    storage.extend_from_slice(&entry.name);
                    (self.namespace, entry.name.clone(), storage)
                }))
                .collect()
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        interp::{Code, Interp},
        obj::Owned,
    };
    const CASES: &[(&str, &[u8], &[u8])] = &[
        ("PLAIN", b"k", b"k"),
        ("RAW_ZERO", b"k\0tail", b"k\0tail"),
        ("QUALIFIED_AFTER_ZERO", b"k\0::tail", b"k\0::tail"),
        ("SURROGATE", b"k\xed\xa0\x80", b"k\xed\xa0\x80"),
        ("OPAQUE_FF", b"k\xff", b"k\xff"),
        ("PREFIX_DECLARATION_NEGATIVE", b"k", b"k\0tail"),
    ];

    fn call(interp: &mut Interp, words: &[&[u8]]) -> Code {
        let original: Vec<_> = words
            .iter()
            .map(|word| Owned::fresh(crate::obj::new_string_bytes(word)))
            .collect();
        let arguments: Vec<_> = original.iter().map(Owned::as_ptr).collect();
        interp.dispatch(&arguments)
    }

    fn assert_row(interp: &Interp, code: Code, rows: &str, label: &str) {
        assert!(
            !interp.host_refusal_pending(),
            "{label}: native access {:?}, compilation admission {:?}",
            interp.native_access_refusal(),
            interp.native_compilation_admission_error(),
        );
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
            code.as_int(),
            fields[1].parse::<i64>().unwrap(),
            "{label}: {:?}",
            interp.result_bytes()
        );
        let actual = interp.result_bytes();
        let protocol = interp
            .native_invocation_dialect()
            .native_string_protocol()
            .unwrap();
        let observed = if label.ends_with("_OBJECT_VARS_AFTER_STATIC")
            || label.ends_with("_OBJECT_VARS_AFTER_DYNAMIC")
        {
            protocol.legacy_eval_result_input(&actual).unwrap()
        } else {
            &actual
        };
        assert_eq!(observed, expected, "{label}");
        assert!(!interp.host_refusal_pending(), "{label}");
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
                include_str!(
                    "../../../../rust/tcl-vm/tests/data/native_oo_variable_resolver/8.6.18/stdout.tsv"
                ),
                include_str!(
                    "../../../../rust/tcl-vm/tests/data/native_oo_resident_keys/v5/8.6.18/stdout.tsv"
                ),
            ),
            (
                "tcl9.0",
                include_str!(
                    "../../../../rust/tcl-vm/tests/data/native_oo_variable_resolver/9.0.4/stdout.tsv"
                ),
                include_str!(
                    "../../../../rust/tcl-vm/tests/data/native_oo_resident_keys/v5/9.0.4/stdout.tsv"
                ),
            ),
            (
                "tcl9.1",
                include_str!(
                    "../../../../rust/tcl-vm/tests/data/native_oo_variable_resolver/9.1.0/stdout.tsv"
                ),
                include_str!(
                    "../../../../rust/tcl-vm/tests/data/native_oo_resident_keys/v5/9.1.0/stdout.tsv"
                ),
            ),
        ] {
            for &(label, declaration, primary) in CASES {
                let mut interp = Interp::with_native_core(
                    crate::interp::default_host(),
                    crate::environment::profile_for_dialect(engine),
                    tcl_registry::special_vars::NativeBootstrapInputs::default(),
                )
                .unwrap();
                assert_eq!(
                    interp.eval_str(b"oo::class create C {};C create c"),
                    Code::Ok
                );
                let code = call(
                    &mut interp,
                    &[b"oo::define", b"C", b"variable", declaration],
                );
                assert_row(&interp, code, rows, &format!("{label}_DECLARE"));
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
                            &mut interp,
                            &[b"oo::define", b"C", b"method", name, parameters, body]
                        ),
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
                    let code = call(&mut interp, &words);
                    assert_row(&interp, code, rows, &format!("{label}_{suffix}"));
                    if suffix == "OBJECT_VARS_AFTER_STATIC" {
                        assert_row(
                            &interp,
                            code,
                            counted_rows,
                            &format!("{label}_OBJECT_VARS_AFTER_STATIC_OBJECT_VECTOR"),
                        );
                    }
                }
            }
        }
    }
}
