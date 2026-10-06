// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual return-option grammar and independently authored logical providers.

use tcl_cmd_core::return_options::ReturnOptionsProtocol;
use tcl_dialect::TclVersion;
use tcl_syntax::native_string::NativeStringProtocol;

/// Explicit logical return provider; this does not attest a vendor engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogicalReturnOptionsProvider {
    /// Authored F5 logical core return semantics without native engine attestation.
    Tcl84CoreSimulation,
}

/// The reached bytecode operation applying an original options object.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeReturnOptionsApplication {
    /// `RETURN_IMM` supplies already-merged options and explicit code/level.
    Immediate,
    /// `SYNTAX` supplies already-merged options with error code and zero level.
    Syntax,
    /// `RETURN_STK` reaches the original List conversion and options merge.
    Stack,
}

/// Selected native compiler's message operand for an authentic parse failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeSyntaxMessageAllocation {
    /// C85/C86 register a fresh unshared counted String.
    UnsharedString,
    /// C90 registers the message through its interpreter literal world.
    RegisteredString,
    /// C91 retains the parser's same result object with `TclAddLiteralObj`.
    OriginalObject,
}

/// Actual C return-instruction recipe, independent of command argv grammar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeReturnOptionsApplicationProtocol {
    strings: NativeStringProtocol,
    application: NativeReturnOptionsApplication,
}
impl NativeReturnOptionsApplicationProtocol {
    /// Actual original opcode-name descriptor used by `TclGetInnerContext`.
    #[must_use]
    pub const fn inner_context_name(
        self,
    ) -> Option<tcl_syntax::native_instruction_name::NativeInstructionName> {
        use tcl_syntax::native_instruction_name::{
            NativeInstructionName, NativeReturnInstructionName,
        };
        let instruction = match self.application {
            NativeReturnOptionsApplication::Immediate => NativeReturnInstructionName::Immediate,
            NativeReturnOptionsApplication::Syntax => NativeReturnInstructionName::Syntax,
            NativeReturnOptionsApplication::Stack => return None,
        };
        NativeInstructionName::for_return(self.strings, instruction)
    }
    /// Original message allocation selected by a reached Syntax compiler.
    #[must_use]
    pub const fn syntax_message_allocation(self) -> Option<NativeSyntaxMessageAllocation> {
        if !matches!(self.application, NativeReturnOptionsApplication::Syntax) {
            return None;
        }
        match self.strings {
            NativeStringProtocol::C(TclVersion::V8_5 | TclVersion::V8_6) => {
                Some(NativeSyntaxMessageAllocation::UnsharedString)
            }
            NativeStringProtocol::C(TclVersion::V9_0) => {
                Some(NativeSyntaxMessageAllocation::RegisteredString)
            }
            NativeStringProtocol::C(TclVersion::V9_1) => {
                Some(NativeSyntaxMessageAllocation::OriginalObject)
            }
            _ => None,
        }
    }
    /// C9.1's `TclCompileSyntaxError` retains its original message in the
    /// compiled options' `-errorinfo` member as well as the object array.
    #[must_use]
    pub const fn syntax_options_share_message(self) -> bool {
        matches!(self.application, NativeReturnOptionsApplication::Syntax)
            && matches!(self.strings, NativeStringProtocol::C(TclVersion::V9_1))
    }
    /// C85 Syntax retains the private error-stack member; C86+ remove it to
    /// avoid retaining a stack cycle in the original compiled options operand.
    #[must_use]
    pub const fn syntax_retains_error_stack(self) -> bool {
        matches!(self.application, NativeReturnOptionsApplication::Syntax)
            && matches!(self.strings, NativeStringProtocol::C(TclVersion::V8_5))
    }
    /// Original objects use this authenticated string/Dictionary recipe.
    #[must_use]
    pub const fn strings(self) -> NativeStringProtocol {
        self.strings
    }
    /// `TclProcessReturn` retains the supplied merged Dictionary header.
    #[must_use]
    pub const fn retains_merged_header(self) -> bool {
        !matches!(self.application, NativeReturnOptionsApplication::Stack)
    }
    /// `SYNTAX` clears the already-logged flag after result publication.
    #[must_use]
    pub const fn clears_logged_after_result(self) -> bool {
        matches!(self.application, NativeReturnOptionsApplication::Syntax)
    }
    /// `TclCompileSyntaxError` removes the live error-stack option in C86+.
    #[must_use]
    pub const fn omits_syntax_error_stack(self) -> bool {
        matches!(self.application, NativeReturnOptionsApplication::Syntax)
            && matches!(
                self.strings,
                NativeStringProtocol::C(TclVersion::V8_6 | TclVersion::V9_0 | TclVersion::V9_1)
            )
    }
    /// C85/86 pass length -1 for the options getter's empty error-info
    /// append. Its wrapper copies a shared header before discovering no bytes.
    #[must_use]
    pub const fn options_getter_copies_shared_info(self) -> bool {
        matches!(
            self.strings,
            NativeStringProtocol::C(TclVersion::V8_5 | TclVersion::V8_6)
        )
    }
    /// The modern options getter initializes errorLine with absent errorInfo.
    #[must_use]
    pub const fn resets_error_line_without_info(self) -> bool {
        matches!(
            self.strings,
            NativeStringProtocol::C(TclVersion::V8_6 | TclVersion::V9_0 | TclVersion::V9_1)
        )
    }
    /// Check the explicit control fields licensed by this reached instruction.
    #[must_use]
    pub fn accepts_control(self, code: i32, level: i64) -> bool {
        i32::try_from(level).is_ok()
            && level >= 0
            && (!matches!(self.application, NativeReturnOptionsApplication::Syntax)
                || (code == 1 && level == 0))
    }
}

/// Retained supported-backend declaration used by the native completion-code getter.
#[must_use]
pub fn completion_code_table() -> crate::native_index_lookup::NativeStaticIndexTable {
    static WORDS: &[&str] = &["ok", "error", "return", "break", "continue"];
    crate::native_index_lookup::NativeStaticIndexTable::supported_backend(WORDS)
}

impl crate::InvocationDialect {
    /// Select the actual C instruction application boundary. Tcl 8.4 and Jim
    /// do not receive the modern private Dictionary/header protocol.
    #[must_use]
    pub fn native_return_options_application(
        self,
        application: NativeReturnOptionsApplication,
    ) -> Option<NativeReturnOptionsApplicationProtocol> {
        let strings = self.native_string_protocol()?;
        if !matches!(strings, NativeStringProtocol::C(version) if version >= TclVersion::V8_5) {
            return None;
        }
        Some(NativeReturnOptionsApplicationProtocol {
            strings,
            application,
        })
    }
    /// Completion-code and return-option behavior of the actual original objects.
    #[must_use]
    pub fn return_options_protocol(self) -> Option<ReturnOptionsProtocol> {
        Some(match self.native_string_protocol()? {
            NativeStringProtocol::Jim084 => ReturnOptionsProtocol::Jim084,
            NativeStringProtocol::C(TclVersion::V8_4) => ReturnOptionsProtocol::Tcl84,
            NativeStringProtocol::C(TclVersion::V8_5) => ReturnOptionsProtocol::Tcl85,
            NativeStringProtocol::C(_) => ReturnOptionsProtocol::Tcl86Plus,
        })
    }

    /// Select an explicit logical provider independently of actual native proof.
    #[must_use]
    pub fn logical_return_options_protocol(
        self,
        provider: LogicalReturnOptionsProvider,
    ) -> Option<ReturnOptionsProtocol> {
        self.authored_f5_tcl84_core()?;
        match provider {
            LogicalReturnOptionsProvider::Tcl84CoreSimulation => Some(ReturnOptionsProtocol::Tcl84),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn actual_instruction_options_distinguish_merged_headers_from_stack_merge() {
        use NativeReturnOptionsApplication::{Immediate, Stack, Syntax};
        for version in TclVersion::ALL {
            let dialect = crate::InvocationDialect::for_version(version);
            for purpose in [Immediate, Syntax, Stack] {
                let selected = dialect.native_return_options_application(purpose);
                if version == TclVersion::V8_4 {
                    assert!(selected.is_none());
                    continue;
                }
                let selected = selected.unwrap();
                assert_eq!(selected.strings(), NativeStringProtocol::C(version));
                assert_eq!(selected.retains_merged_header(), purpose != Stack);
                assert_eq!(selected.clears_logged_after_result(), purpose == Syntax);
                assert_eq!(
                    selected.syntax_options_share_message(),
                    purpose == Syntax && version == TclVersion::V9_1,
                );
                assert_eq!(
                    selected.syntax_message_allocation(),
                    if purpose == Syntax {
                        Some(match version {
                            TclVersion::V8_5 | TclVersion::V8_6 => {
                                NativeSyntaxMessageAllocation::UnsharedString
                            }
                            TclVersion::V9_0 => NativeSyntaxMessageAllocation::RegisteredString,
                            TclVersion::V9_1 => NativeSyntaxMessageAllocation::OriginalObject,
                            TclVersion::V8_4 => unreachable!("modern selected return protocol"),
                        })
                    } else {
                        None
                    }
                );
                assert!(selected.accepts_control(1, 0));
                assert_eq!(selected.accepts_control(37, 2), purpose != Syntax);
                assert!(!selected.accepts_control(1, -1));
                assert!(!selected.accepts_control(1, i64::from(i32::MAX) + 1));
            }
        }
        for dialect in [
            crate::InvocationDialect::of_point(tcl_dialect::model::DialectPoint::canonical(
                tcl_dialect::model::Release::JIM_0_84,
            )),
            crate::InvocationDialect::of_profile(tcl_dialect::DialectProfile::irules()),
        ] {
            assert!(
                dialect
                    .native_return_options_application(Immediate)
                    .is_none()
            );
        }
    }

    #[test]
    fn actual_return_recipes_reject_vendor_compatibility_and_conflicts() {
        use crate::InvocationDialect;
        use tcl_dialect::model::{DialectPoint, Family, Release};
        for version in TclVersion::ALL {
            let mut dialect = InvocationDialect::for_version(version);
            assert!(dialect.return_options_protocol().is_some());
            dialect.native_family = Some(Family::F5Irules);
            assert_eq!(dialect.return_options_protocol(), None);
            dialect.native_family = None;
            assert_eq!(dialect.return_options_protocol(), None);
        }
        let jim = InvocationDialect::of_point(DialectPoint::canonical(Release::JIM_0_84));
        assert_eq!(
            jim.return_options_protocol(),
            Some(ReturnOptionsProtocol::Jim084)
        );
        let mut conflict = InvocationDialect::for_version(TclVersion::V8_6);
        conflict.core_point = Some(DialectPoint::canonical(Release::TCL_9_0));
        assert_eq!(conflict.return_options_protocol(), None);
        let logical = InvocationDialect::of_profile(tcl_dialect::DialectProfile::irules());
        assert_eq!(logical.return_options_protocol(), None);
        assert_eq!(
            logical
                .logical_return_options_protocol(LogicalReturnOptionsProvider::Tcl84CoreSimulation),
            Some(ReturnOptionsProtocol::Tcl84)
        );
    }
}
