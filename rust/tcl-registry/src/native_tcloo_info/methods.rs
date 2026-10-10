// SPDX-License-Identifier: AGPL-3.0-or-later
//! `TclOO` method options; target lookup and result-name objects are independent.

use super::{
    CmdError, InfoOoEnsembleKind, InvocationDialect, NativeStringProtocol,
    NativeTclooVariableInfoProtocol, TclVersion,
};
use crate::prelude::{ArgValue, OptionSpec, OptionValue};
use tcl_cmd_core::prefix::OptionTable;

const SCOPE_VALUES: [ArgValue; 3] = [
    ArgValue {
        value: "private",
        detail: "Only true-private local methods.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "public",
        detail: "Only exported local methods.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "unexported",
        detail: "Only ordinary unexported local methods.",
        ..ArgValue::DEFAULT
    },
];

/// The same release-filtered catalogue used by the native option table.
pub const METHOD_INFO_OPTIONS: [OptionSpec; 4] = [
    OptionSpec {
        name: "-all",
        detail: "Include inherited methods unless an explicit scope is selected.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-localprivate",
        detail: "Select the native direct-instance private method flag.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-private",
        detail: "Include unexported methods; this does not expose true-private methods.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-scope",
        value: OptionValue::enumerated(&SCOPE_VALUES, true, "scope"),
        detail: "Select exactly one local method visibility class.",
        surface: Some(tcl_dialect::model::SpecSurface::TCL90_PLUS),
        ..OptionSpec::DEFAULT
    },
];
const LEGACY_NAMES: [&str; 3] = [
    METHOD_INFO_OPTIONS[0].name,
    METHOD_INFO_OPTIONS[1].name,
    METHOD_INFO_OPTIONS[2].name,
];
const MODERN_NAMES: [&str; 4] = [
    METHOD_INFO_OPTIONS[0].name,
    METHOD_INFO_OPTIONS[1].name,
    METHOD_INFO_OPTIONS[2].name,
    METHOD_INFO_OPTIONS[3].name,
];
const SCOPES: [&str; 3] = [
    SCOPE_VALUES[0].value,
    SCOPE_VALUES[1].value,
    SCOPE_VALUES[2].value,
];

/// Release-selected method enumeration options, without an OO target grant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeTclooMethodInfoProtocol {
    version: TclVersion,
}

/// The selected filter and recursion recipe; neither asserts method existence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeTclooMethodInfoSelection {
    version: TclVersion,
    recurse: bool,
    flag: u8,
    explicit_scope: bool,
}

impl NativeTclooMethodInfoProtocol {
    /// Select only an independently identified C release that provides `TclOO`.
    #[must_use]
    pub fn select(dialect: InvocationDialect) -> Option<Self> {
        NativeTclooVariableInfoProtocol::select(dialect).map(|p| Self { version: p.version })
    }

    /// The native wrong-arity suffix; check target presence before parsing options.
    #[must_use]
    pub const fn usage(kind: InfoOoEnsembleKind) -> &'static str {
        match kind {
            InfoOoEnsembleKind::Object => "info object methods objName ?-option value ...?",
            InfoOoEnsembleKind::Class => "info class methods className ?-option value ...?",
        }
    }

    fn options(self) -> OptionTable<'static> {
        OptionTable::abbreviating(
            "option",
            if self.version >= TclVersion::V9_0 {
                &MODERN_NAMES
            } else {
                &LEGACY_NAMES
            },
        )
    }

    /// Parse original option objects sequentially after the target was found.
    /// Each Index operation stays with `ValueOps`' independently selected native
    /// getter/cache/failure owner. Later operands are untouched after an error.
    ///
    /// # Errors
    /// A getter/Index refusal, unmatched option/scope, or missing scope value.
    pub fn parse_original<O: tcl_syntax::value::ValueOps>(
        self,
        ops: &mut O,
        args: &[O::Value],
    ) -> Result<NativeTclooMethodInfoSelection, CmdError> {
        self.parse_with(args.len(), |index, scope| {
            let table = if scope {
                OptionTable::abbreviating("scope", &SCOPES)
            } else {
                self.options()
            };
            table.index_of_original(ops, &args[index])
        })
    }

    fn parse_with(
        self,
        count: usize,
        mut lookup: impl FnMut(usize, bool) -> Result<usize, CmdError>,
    ) -> Result<NativeTclooMethodInfoSelection, CmdError> {
        let mut result = NativeTclooMethodInfoSelection {
            version: self.version,
            recurse: false,
            flag: 1,
            explicit_scope: false,
        };
        let mut scope = None;
        let mut index = 0;
        while index < count {
            match lookup(index, false)? {
                0 => result.recurse = true,
                1 => result.flag = 2,
                2 => result.flag = 0,
                3 => {
                    index += 1;
                    if index == count {
                        return Err(CmdError::with_error_code_bytes(
                            b"missing option for -scope".to_vec(),
                            b"TCL ARGUMENT MISSING".to_vec(),
                        )
                        .with_native_string_result(NativeStringProtocol::C(self.version)));
                    }
                    scope = Some(lookup(index, true)?);
                }
                _ => unreachable!("the selected TclOO table has at most four options"),
            }
            index += 1;
        }
        if let Some(scope) = scope {
            result.recurse = false;
            result.explicit_scope = true;
            result.flag = match scope {
                0 => 0x20,
                1 => 1,
                2 => 0,
                _ => unreachable!("the native scope table has three entries"),
            };
        }
        Ok(result)
    }
}

impl NativeTclooMethodInfoSelection {
    /// Whether the selected native handler invokes the recursive roster owner.
    #[must_use]
    pub const fn recurse(self) -> bool {
        self.recurse
    }

    /// Whether recursion requests unexported methods, independent of local filtering.
    #[must_use]
    pub const fn includes_unexported(self) -> bool {
        self.flag & 1 == 0
    }

    /// The local table filter on independently known native visibility fields.
    /// This supplies no body/name object, dispatch, or flags inventory authority.
    #[must_use]
    pub const fn matches_local(
        self,
        public: bool,
        direct_instance_private: bool,
        true_private: bool,
    ) -> bool {
        let flags =
            (public as u8) | ((direct_instance_private as u8) << 1) | ((true_private as u8) << 5);
        let mask = if self.explicit_scope {
            0x23
        } else if matches!(self.version, TclVersion::V9_0 | TclVersion::V9_1) {
            self.flag | 0x20
        } else {
            self.flag
        };
        flags & mask == self.flag
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn parse(
        version: TclVersion,
        args: &[&[u8]],
    ) -> Result<NativeTclooMethodInfoSelection, CmdError> {
        let selected =
            NativeTclooMethodInfoProtocol::select(InvocationDialect::for_version(version)).unwrap();
        selected.parse_with(args.len(), |i, scope| {
            let table = if scope {
                OptionTable::abbreviating("scope", &SCOPES)
            } else {
                selected.options()
            };
            let word = tcl_core_types::c_string_extent(args[i]);
            table.index_of_cmd(word)
        })
    }
    #[test]
    fn method_options_keep_cstring_prefix_and_sequential_scope_selection() {
        // Source proof: naming.tcloo.method-info-option-source
        // docs/design/analysis/name-resolution-proofs/method-info-option-source.md
        for version in [TclVersion::V8_6, TclVersion::V9_0, TclVersion::V9_1] {
            assert!(
                parse(version, &[b"-p\0\xff"])
                    .unwrap()
                    .includes_unexported()
            );
            assert!(parse(version, &[b"-private\xc0\x80"]).is_err());
            assert!(parse(version, &[b"-"]).is_err());
            let local = parse(version, &[b"-localprivate"]).unwrap();
            assert!(!local.matches_local(false, false, false));
            assert!(local.matches_local(false, true, false));
            assert!(
                parse(version, &[b"-localprivate", b"-private"])
                    .unwrap()
                    .matches_local(false, false, false)
            );
            if version >= TclVersion::V9_0 {
                let scoped = parse(
                    version,
                    &[
                        b"-scope",
                        b"public",
                        b"-scope\0x",
                        b"unexported\0tail",
                        b"-all",
                    ],
                )
                .unwrap();
                assert!(!scoped.recurse());
                assert!(scoped.matches_local(false, false, false));
                assert!(!scoped.matches_local(true, false, false));
                assert!(!scoped.matches_local(false, false, true));
                assert!(parse(version, &[b"-scope", b"p"]).is_err());
                assert_eq!(
                    parse(version, &[b"-scope"]).unwrap_err().message_bytes(),
                    b"missing option for -scope"
                );
                assert!(
                    parse(version, &[b"-scope", b"private"])
                        .unwrap()
                        .matches_local(false, false, true)
                );
            } else {
                assert!(parse(version, &[b"-scope", b"public"]).is_err());
            }
        }
        for version in [TclVersion::V8_4, TclVersion::V8_5] {
            assert!(
                NativeTclooMethodInfoProtocol::select(InvocationDialect::for_version(version))
                    .is_none()
            );
        }
    }
}
