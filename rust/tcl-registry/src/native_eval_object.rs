// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original script-object dispatch, independently of script grammar.

use tcl_dialect::TclVersion;
use tcl_syntax::native_object::{NativeObjectCacheSnapshot, NativeObjectSnapshot};

use crate::InvocationDialect;

/// Operation receiving an original script object.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvalObjectPurpose {
    /// The `eval` object's direct evaluation entry.
    Eval,
    /// Evaluation in an explicitly selected variable frame.
    UpLevel,
    /// Evaluation in an activated namespace.
    NamespaceBody,
    /// A namespace body constructed by concatenating multiple written operands.
    NamespaceConcat,
    /// An ordinary control-command body evaluated as an object.
    ControlBody,
}

/// Independently installed logical script-object dispatcher.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LogicalEvalObjectProvider {
    /// Authored F5 Tcl8.4 core evaluation-object simulation.
    Tcl84CoreSimulation,
}

/// Authenticated native script-object dispatch recipe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeEvalObjectProtocol {
    version: Option<TclVersion>,
}

impl NativeEvalObjectProtocol {
    /// Source objects use the native compiler unless the selected C84 caller
    /// explicitly requests direct evaluation. This does not admit an artifact.
    #[must_use]
    pub const fn compiles_source(self, purpose: EvalObjectPurpose) -> bool {
        match self.version {
            Some(TclVersion::V8_4) => !matches!(
                purpose,
                EvalObjectPurpose::Eval
                    | EvalObjectPurpose::UpLevel
                    | EvalObjectPurpose::NamespaceConcat
            ),
            Some(_) => true,
            None => false,
        }
    }

    /// Whether this recipe authorizes fresh source operands without native literal registration.
    /// Jim's Script parser owns its tokens separately and cannot use this C84 door.
    #[must_use]
    pub const fn permits_direct_source_operands(
        self,
        purpose: EvalObjectPurpose,
        strings: tcl_syntax::native_string::NativeStringProtocol,
    ) -> bool {
        matches!(self.version, Some(TclVersion::V8_4))
            && matches!(
                strings,
                tcl_syntax::native_string::NativeStringProtocol::C(TclVersion::V8_4)
            )
            && !self.compiles_source(purpose)
    }

    /// C84 compiled bodies fail before entering a clean command prefix.
    #[must_use]
    pub const fn parse_failure_precedes_commands(self, purpose: EvalObjectPurpose) -> bool {
        matches!(self.version, Some(TclVersion::V8_4)) && self.compiles_source(purpose)
    }

    /// Public C source evaluation consumes its command-log flag on exit.
    /// Shared command substitutions and inline bodies retain their own unwind flag.
    #[must_use]
    pub const fn clears_public_source_error_logged(self) -> bool {
        self.version.is_some()
    }

    /// Whether the retained primary List can dispatch its original elements.
    /// This query neither generates a string nor parses an ordinary string.
    #[must_use]
    pub fn dispatches_list(
        self,
        purpose: EvalObjectPurpose,
        object: &NativeObjectSnapshot,
    ) -> bool {
        let NativeObjectCacheSnapshot::List { canonical, .. } = object.cache else {
            return false;
        };
        match self.version {
            Some(TclVersion::V8_4) => {
                matches!(
                    purpose,
                    EvalObjectPurpose::Eval | EvalObjectPurpose::NamespaceConcat
                ) && object.resident.is_none()
            }
            Some(_) => object.resident.is_none() || canonical,
            None => object.resident.is_none(),
        }
    }
}

impl InvocationDialect {
    /// Select the current invocation from the interpreter's installed capabilities.
    /// An actual core invocation uses its own recipe; a logical invocation must
    /// independently match the installed provider's authored domain. Keeping a
    /// provider installed does not override nested actual-core activations.
    #[must_use]
    pub fn invocation_eval_object_protocol(
        self,
        installed: Option<LogicalEvalObjectProvider>,
    ) -> Option<NativeEvalObjectProtocol> {
        self.native_eval_object_protocol()
            .or_else(|| self.eval_object_protocol(installed))
    }

    /// Select an explicit logical dispatcher without borrowing the physical
    /// host's compiler, string updater or native object-type authority.
    #[must_use]
    pub fn eval_object_protocol(
        self,
        logical: Option<LogicalEvalObjectProvider>,
    ) -> Option<NativeEvalObjectProtocol> {
        match logical {
            Some(LogicalEvalObjectProvider::Tcl84CoreSimulation) => {
                self.authored_f5_tcl84_core()?;
                Some(NativeEvalObjectProtocol {
                    version: Some(TclVersion::V8_4),
                })
            }
            None => self.native_eval_object_protocol(),
        }
    }
    /// Actual evaluation-object recipe. Compatible vendor releases and lexer
    /// overrides cannot provide native dispatch or canonical-list authority.
    #[must_use]
    pub fn native_eval_object_protocol(self) -> Option<NativeEvalObjectProtocol> {
        self.native_scalar_getter_protocol()
            .map(|protocol| NativeEvalObjectProtocol {
                version: protocol.tcl_version(),
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_source_exit_consumes_only_the_actual_c_command_log_flag() {
        for version in TclVersion::ALL {
            assert!(
                InvocationDialect::for_version(version)
                    .native_eval_object_protocol()
                    .unwrap()
                    .clears_public_source_error_logged()
            );
        }
        let jim = InvocationDialect::of_profile(
            crate::model::ingress::resolve_environment("jim").analyser_profile(),
        );
        assert!(
            !jim.native_eval_object_protocol()
                .unwrap()
                .clears_public_source_error_logged()
        );
        let mut unknown = InvocationDialect::for_version(TclVersion::V8_6);
        unknown.core_point = None;
        unknown.native_family = None;
        assert!(unknown.native_eval_object_protocol().is_none());
    }

    #[test]
    fn installed_logical_dispatcher_keeps_actual_invocations_independent() {
        let installed = Some(LogicalEvalObjectProvider::Tcl84CoreSimulation);
        for version in TclVersion::ALL {
            let dialect = InvocationDialect::for_version(version);
            assert_eq!(
                dialect.invocation_eval_object_protocol(installed),
                dialect.native_eval_object_protocol(),
            );
            assert!(dialect.eval_object_protocol(installed).is_none());
        }
        let jim = InvocationDialect::of_profile(
            crate::model::ingress::resolve_environment("jim").unit_profile(),
        );
        assert_eq!(
            jim.invocation_eval_object_protocol(installed),
            jim.native_eval_object_protocol(),
        );
        let logical = InvocationDialect::of_profile(tcl_dialect::DialectProfile::irules());
        assert_eq!(
            logical.invocation_eval_object_protocol(installed),
            logical.eval_object_protocol(installed),
        );
        assert!(logical.invocation_eval_object_protocol(None).is_none());
    }

    #[test]
    fn list_dispatch_requires_the_selected_purpose_and_primary_receipt() {
        let mut object = NativeObjectSnapshot {
            resident: None,
            storage: None,
            cache: NativeObjectCacheSnapshot::List {
                length: 1,
                canonical: true,
            },
        };
        let old = InvocationDialect::for_version(TclVersion::V8_4)
            .native_eval_object_protocol()
            .unwrap();
        assert!(old.dispatches_list(EvalObjectPurpose::Eval, &object));
        assert!(!old.dispatches_list(EvalObjectPurpose::UpLevel, &object));
        assert!(!old.dispatches_list(EvalObjectPurpose::NamespaceBody, &object));
        assert!(old.dispatches_list(EvalObjectPurpose::NamespaceConcat, &object));
        object.resident = Some(std::rc::Rc::from(&b"set"[..]));
        assert!(!old.dispatches_list(EvalObjectPurpose::Eval, &object));
        assert!(!old.dispatches_list(EvalObjectPurpose::NamespaceConcat, &object));
        let later = InvocationDialect::for_version(TclVersion::V8_6)
            .native_eval_object_protocol()
            .unwrap();
        assert!(later.dispatches_list(EvalObjectPurpose::UpLevel, &object));
        object.cache = NativeObjectCacheSnapshot::List {
            length: 1,
            canonical: false,
        };
        assert!(!later.dispatches_list(EvalObjectPurpose::Eval, &object));
    }
    #[test]
    fn jim_and_logical_eval_keep_independent_original_object_recipes() {
        let mut object = NativeObjectSnapshot {
            resident: None,
            storage: None,
            cache: NativeObjectCacheSnapshot::List {
                length: 1,
                canonical: true,
            },
        };
        let jim = InvocationDialect::of_profile(
            crate::model::ingress::resolve_environment("jim").unit_profile(),
        )
        .native_eval_object_protocol()
        .unwrap();
        assert!(jim.dispatches_list(EvalObjectPurpose::UpLevel, &object));
        object.resident = Some(std::rc::Rc::from(&b"set"[..]));
        assert!(!jim.dispatches_list(EvalObjectPurpose::Eval, &object));
        let f5 = InvocationDialect::of_profile(tcl_dialect::DialectProfile::irules());
        assert!(f5.eval_object_protocol(None).is_none());
        let provider = LogicalEvalObjectProvider::Tcl84CoreSimulation;
        let logical = f5.eval_object_protocol(Some(provider)).unwrap();
        assert!(!logical.dispatches_list(EvalObjectPurpose::Eval, &object));
        object.resident = None;
        assert!(logical.dispatches_list(EvalObjectPurpose::Eval, &object));
        assert!(!logical.dispatches_list(EvalObjectPurpose::UpLevel, &object));
        assert!(
            InvocationDialect::for_version(TclVersion::V9_0)
                .eval_object_protocol(Some(provider))
                .is_none()
        );
    }
}
