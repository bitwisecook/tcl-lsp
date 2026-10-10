// SPDX-License-Identifier: AGPL-3.0-or-later
//! Selected vwait operand grammar, separate from notifier or observer capability.

use crate::{ArgRole, InvocationArguments, InvocationDialect};
use tcl_dialect::TclVersion;

/// Original argv boundary accepted by the basic single-name wait owner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeVwaitForm {
    /// One original variable object, interpreted in the global frame.
    Basic,
    /// Requires a separate extended parser and event capability; this does not
    /// assert that the supplied options or operands are valid.
    Extended,
    /// The selected legacy or Jim operand count is invalid.
    WrongArity,
}

/// Native command grammar selected independently of variable-cell currency.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeVwaitProtocol {
    /// Tcl 8 accepts exactly one variable name, including option-like names.
    CLegacy,
    /// Tcl 9 retains the one-name exception except for `CString` `--`.
    CExtended(TclVersion),
    /// Jim accepts one name, optional exact `-signal`, and an optional script.
    Jim084,
}

const C_OPTIONS: &[&str] = &[
    "-all",
    "-extended",
    "-nofileevents",
    "-noidleevents",
    "-notimerevents",
    "-nowindowevents",
    "-readable",
    "-timeout",
    "-variable",
    "-writable",
    "--",
];

impl NativeVwaitProtocol {
    /// Classify only the basic owner's boundary, obtaining an original operand
    /// only where the selected command examines it. No names are looked up.
    ///
    /// # Errors
    /// Preserves an unavailable reached original string operand.
    pub fn form<B: AsRef<[u8]>, E>(
        self,
        count: usize,
        mut original_bytes: impl FnMut(usize) -> Result<B, E>,
    ) -> Result<NativeVwaitForm, E> {
        // Implementation contract: naming.event.original-vwait-operand-boundaries
        // docs/design/analysis/name-resolution-proofs/event-original-vwait-operand-boundaries.md
        Ok(match self {
            Self::CLegacy => {
                if count == 1 {
                    NativeVwaitForm::Basic
                } else {
                    NativeVwaitForm::WrongArity
                }
            }
            Self::CExtended(_) => {
                if count == 1
                    && tcl_core_types::c_string_extent(original_bytes(0)?.as_ref()) != b"--"
                {
                    NativeVwaitForm::Basic
                } else {
                    NativeVwaitForm::Extended
                }
            }
            Self::Jim084 => match count {
                1 => NativeVwaitForm::Basic,
                2 => NativeVwaitForm::Extended,
                3 if tcl_core_types::c_string_extent(original_bytes(0)?.as_ref()) == b"-signal" => {
                    NativeVwaitForm::Extended
                }
                _ => NativeVwaitForm::WrongArity,
            },
        })
    }

    /// Selected usage after a genuine arity rejection, not after an unavailable
    /// extended operation. Tcl 9's extended forms have no legacy arity rejection.
    #[must_use]
    pub const fn wrong_arguments(self) -> &'static str {
        match self {
            Self::Jim084 => "wrong # args: should be \"vwait ?-signal? name ?script?\"",
            Self::CLegacy | Self::CExtended(_) => "wrong # args: should be \"vwait name\"",
        }
    }

    /// Source role positions from the complete selected argv grammar. Names and
    /// body words retain their own producers; this supplies no global cell,
    /// observer, entered script, normal completion or notifier authority.
    #[must_use]
    pub fn source_roles(self, arguments: InvocationArguments<'_>) -> Option<Vec<(u8, ArgRole)>> {
        // Implementation contract: naming.event.original-vwait-operand-boundaries
        // docs/design/analysis/name-resolution-proofs/event-original-vwait-operand-boundaries.md
        let count = arguments.exact_argv_len()?;
        if count > usize::from(u8::MAX) + 1 {
            return None;
        }
        let bytes = |index| {
            arguments
                .native_bytes_at(index)
                .or_else(|| arguments.literal_at(index).map(str::as_bytes))
        };
        match self {
            Self::CLegacy => (count == 1).then(|| vec![(0, ArgRole::VarWrite)]),
            Self::Jim084 => {
                let signal = count > 1 && tcl_core_types::c_string_extent(bytes(0)?) == b"-signal";
                let name = usize::from(signal);
                let remaining = count.checked_sub(name)?;
                if !(1..=2).contains(&remaining) {
                    return None;
                }
                let mut roles = vec![(u8::try_from(name).ok()?, ArgRole::VarWrite)];
                if remaining == 2 {
                    roles.push((u8::try_from(name + 1).ok()?, ArgRole::Body));
                }
                Some(roles)
            }
            Self::CExtended(version) => {
                if count == 1 && tcl_core_types::c_string_extent(bytes(0)?) != b"--" {
                    return Some(vec![(0, ArgRole::VarWrite)]);
                }
                let index =
                    InvocationDialect::for_version(version).native_index_lookup_protocol()?;
                let table = crate::native_index_lookup::NativeStaticIndexTable::supported_backend(
                    C_OPTIONS,
                );
                let mut roles = Vec::new();
                let mut cursor = 0;
                while cursor < count {
                    let original = bytes(cursor)?;
                    if tcl_core_types::c_string_extent(original).first() != Some(&b'-') {
                        break;
                    }
                    let option = index
                        .lookup(original, &table, false, "option")
                        .ok()?
                        .index();
                    cursor += 1;
                    match option {
                        0..=5 => {}
                        6 | 7 | 9 => {
                            if cursor >= count {
                                return None;
                            }
                            cursor += 1;
                        }
                        8 => {
                            if cursor >= count {
                                return None;
                            }
                            roles.push((u8::try_from(cursor).ok()?, ArgRole::VarWrite));
                            cursor += 1;
                        }
                        10 => break,
                        _ => unreachable!("selected static option table"),
                    }
                }
                roles.extend(
                    (cursor..count)
                        .map(|i| (u8::try_from(i).expect("bounded ordinal"), ArgRole::VarWrite)),
                );
                Some(roles)
            }
        }
    }
}

impl InvocationDialect {
    /// Actual C or pinned Jim event grammar; missing native axes decline.
    #[must_use]
    pub fn native_vwait_protocol(self) -> Option<NativeVwaitProtocol> {
        self.native_event_protocol()
            .map(super::native_event::NativeEventProtocol::vwait)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn original_vwait_forms_keep_legacy_names_and_extended_refusal_separate() {
        // Implementation contract: naming.event.original-vwait-operand-boundaries
        // docs/design/analysis/name-resolution-proofs/event-original-vwait-operand-boundaries.md
        for version in TclVersion::ALL {
            let protocol = InvocationDialect::for_version(version)
                .native_vwait_protocol()
                .unwrap();
            for name in [
                b"-all".as_slice(),
                b"-signal",
                b"--\xc0\x80tail",
                b"name\xed\xa0\x80",
            ] {
                assert_eq!(
                    protocol.form(1, |_| Ok::<_, ()>(name)).unwrap(),
                    NativeVwaitForm::Basic
                );
            }
            for name in [b"--".as_slice(), b"--\0tail"] {
                assert_eq!(
                    protocol.form(1, |_| Ok::<_, ()>(name)).unwrap(),
                    if version >= TclVersion::V9_0 {
                        NativeVwaitForm::Extended
                    } else {
                        NativeVwaitForm::Basic
                    }
                );
            }
            let mut reads = 0;
            let result = protocol
                .form(2, |_| {
                    reads += 1;
                    Ok::<_, ()>(b"name".as_slice())
                })
                .unwrap();
            assert_eq!(reads, 0);
            assert_eq!(
                result,
                if version >= TclVersion::V9_0 {
                    NativeVwaitForm::Extended
                } else {
                    NativeVwaitForm::WrongArity
                }
            );
        }
        let jim = InvocationDialect::of_profile(
            crate::model::ingress::resolve_environment("jim").unit_profile(),
        )
        .native_vwait_protocol()
        .unwrap();
        assert_eq!(
            jim.form(1, |_| Err::<&[u8], ()>(())).unwrap(),
            NativeVwaitForm::Basic
        );
        assert_eq!(
            jim.form(2, |_| Err::<&[u8], ()>(())).unwrap(),
            NativeVwaitForm::Extended
        );
        assert_eq!(
            jim.form(3, |_| Ok::<_, ()>(b"-signal\0tail".as_slice()))
                .unwrap(),
            NativeVwaitForm::Extended
        );
        assert_eq!(
            jim.form(3, |_| Ok::<_, ()>(b"-sig".as_slice())).unwrap(),
            NativeVwaitForm::WrongArity
        );
    }

    #[test]
    fn original_vwait_roles_select_only_actual_variable_and_script_ordinals() {
        // Implementation contract: naming.event.original-vwait-operand-boundaries
        // docs/design/analysis/name-resolution-proofs/event-original-vwait-operand-boundaries.md
        for version in [TclVersion::V9_0, TclVersion::V9_1] {
            let dialect = InvocationDialect::for_version(version);
            let p = dialect.native_vwait_protocol().unwrap();
            for (args, expected) in [
                (
                    vec!["-timeout", "1", "-variable", "::N::x", "bare"],
                    vec![(3, ArgRole::VarWrite), (4, ArgRole::VarWrite)],
                ),
                (vec!["-all", "--", "-timeout"], vec![(2, ArgRole::VarWrite)]),
                (
                    vec!["-v", "x", "-readable", "channel", "tail"],
                    vec![(1, ArgRole::VarWrite), (4, ArgRole::VarWrite)],
                ),
                (vec!["--"], vec![]),
                (vec!["-all"], vec![(0, ArgRole::VarWrite)]),
            ] {
                assert_eq!(
                    p.source_roles(InvocationArguments::literals(&args).with_dialect(dialect)),
                    Some(expected)
                );
            }
            for args in [
                vec!["-bad", "name"],
                vec!["-variable", "name", "-timeout"],
                vec!["-n", "name"],
            ] {
                assert!(
                    p.source_roles(InvocationArguments::literals(&args).with_dialect(dialect))
                        .is_none()
                );
            }
            let unknown = [
                crate::InvocationWord::Dynamic,
                crate::InvocationWord::Literal("x"),
            ];
            assert!(
                p.source_roles(InvocationArguments::structured(&unknown).with_dialect(dialect))
                    .is_none()
            );
            let named = [
                crate::InvocationWord::Literal("--"),
                crate::InvocationWord::Dynamic,
            ];
            assert_eq!(
                p.source_roles(InvocationArguments::structured(&named).with_dialect(dialect)),
                Some(vec![(1, ArgRole::VarWrite)])
            );
        }
        let dialect = InvocationDialect::of_profile(
            crate::model::ingress::resolve_environment("jim").unit_profile(),
        );
        let p = dialect.native_vwait_protocol().unwrap();
        assert_eq!(
            p.source_roles(
                InvocationArguments::literals(&["-signal", "::N::x", "break"])
                    .with_dialect(dialect)
            ),
            Some(vec![(1, ArgRole::VarWrite), (2, ArgRole::Body)])
        );
        assert_eq!(
            p.source_roles(InvocationArguments::literals(&["name", "break"]).with_dialect(dialect)),
            Some(vec![(0, ArgRole::VarWrite), (1, ArgRole::Body)])
        );
        for version in [TclVersion::V8_4, TclVersion::V8_5, TclVersion::V8_6] {
            let dialect = InvocationDialect::for_version(version);
            assert!(
                dialect
                    .native_vwait_protocol()
                    .unwrap()
                    .source_roles(
                        InvocationArguments::literals(&["name", "script"]).with_dialect(dialect)
                    )
                    .is_none()
            );
        }
    }
    #[test]
    fn original_vwait_owned_registry_roles_do_not_retag_option_values() {
        // Implementation contract: naming.event.original-vwait-operand-boundaries
        // docs/design/analysis/name-resolution-proofs/event-original-vwait-operand-boundaries.md
        for profile in ["tcl9.0", "tcl9.1"] {
            let owner = crate::model::ingress::static_context_for(profile);
            let selected = owner
                .commands()
                .resolve_invocation(
                    "vwait",
                    &["-timeout", "5", "-variable", "::N::x", "--", "bare"],
                    None,
                )
                .unwrap()
                .facts();
            let names = selected
                .arg_roles
                .into_iter()
                .filter(|(_, role)| *role == ArgRole::VarWrite)
                .collect::<Vec<_>>();
            assert!(selected.arg_roles_complete, "{profile}");
            assert_eq!(
                names,
                vec![(3, ArgRole::VarWrite), (5, ArgRole::VarWrite)],
                "{profile}"
            );
        }
        let owner = crate::model::ingress::static_context_for("jim");
        let selected = owner
            .commands()
            .resolve_invocation("vwait", &["-signal", "::N::x", "break"], None)
            .unwrap()
            .facts();
        assert_eq!(
            selected.arg_roles,
            vec![(1, ArgRole::VarWrite), (2, ArgRole::Body)]
        );
        assert!(selected.arg_roles_complete);
    }
}

#[cfg(test)]
#[path = "native_vwait/native_observation_tests.rs"]
mod native_observation_tests;
