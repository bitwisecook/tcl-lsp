// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Dictionary body scopes: mapping, completion and writeback belong together.

use crate::catch_invocation::{CatchInvocation, CatchInvocationSelection, select_catch_invocation};
use crate::completion_route::InvocationCompletionRoute;
use crate::{InvocationArguments, InvocationDialect};
use tcl_dialect::model::Family;

/// Authored body scope grammar, independent of the native compiler hook.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DictionaryScopeSpec {
    /// Explicit key/variable pairs surround one caller-frame body.
    Update,
    /// Every selected dictionary key maps to the same-named caller variable.
    With,
}

/// Missing selected keys affect caller contents before the body starts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DictionaryMissingKey {
    /// C update removes the corresponding caller variable.
    Unset,
    /// Jim's scripted update leaves an existing caller variable untouched.
    Retain,
}

/// Mapping selectors expressed in the resolved invocation's effective argv.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DictionaryScopeBindings {
    /// Ordered key/variable argument positions. Duplicate names remain ordered.
    Pairs(Vec<(usize, usize)>),
    /// Key path selecting a subdictionary whose original keys are mapped.
    AllKeys(Vec<usize>),
}

/// Completion-sensitive epilogue; a process exit never becomes a cleanup edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DictionaryWriteback {
    /// Captured native completions write back before being propagated.
    Captured(CatchInvocation),
    /// Jim with writes back only after ordinary successful body completion.
    NormalOnly,
}

/// Native epilogue failure completion, distinct from body capture policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DictionaryWritebackFailure {
    /// A failed epilogue replaces the body completion with error.
    Error,
    /// Jim with returns OK while retaining the epilogue's error result.
    NormalWithErrorResult,
}

/// Implementation evidence needed independently of the public dispatcher name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DictionaryScopeImplementation {
    /// The selected native primitive owns its complete mapping/writeback protocol.
    NativePrimitive,
    /// A stock scripted wrapper additionally requires its live helper closure.
    ScriptedWrapper,
}

/// A selected scope still requires the shared physical place/value owner.
/// This descriptor never establishes that a dictionary or a mapped key exists.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DictionaryScopePlan {
    /// Required primitive or closed scripted-wrapper implementation proof.
    pub implementation: DictionaryScopeImplementation,
    /// Effective dictionary-variable operand.
    pub dictionary_argument: usize,
    /// Effective evaluated script operand.
    pub body_argument: usize,
    /// Ordered argument mapping; names are read from frozen argv by consumers.
    pub bindings: DictionaryScopeBindings,
    /// Missing-key contents transfer at entry.
    pub missing_key: DictionaryMissingKey,
    /// Actual body-completion paths reaching writeback.
    pub writeback: DictionaryWriteback,
    /// Native completion when a reached writeback fails.
    pub writeback_failure: DictionaryWritebackFailure,
    /// Jim update is a stock scripted procedure adding a return boundary.
    pub procedure_return_boundary: bool,
    /// The selected engine, retained for that procedure completion boundary.
    pub dialect: InvocationDialect,
}

/// Invalid native argv, unavailable engine semantics, or a selected plan.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DictionaryScopeSelection {
    /// The wrapper selects this mapping and completion protocol.
    Selected(DictionaryScopePlan),
    /// Known argument shape is rejected before any body execution.
    Invalid,
    /// Runtime argument shape or native implementation policy is unproved.
    Unknown,
}

impl DictionaryScopeSpec {
    /// Select positions through the central native grammar, never source names.
    #[must_use]
    pub fn select(
        self,
        arguments: InvocationArguments<'_>,
        offset: usize,
    ) -> DictionaryScopeSelection {
        use DictionaryScopeSelection::{Invalid, Selected, Unknown};
        let Some(dialect) = arguments.dialect() else {
            return Unknown;
        };
        let Some(count) = arguments
            .exact_argv_len()
            .and_then(|count| count.checked_sub(offset))
        else {
            return Unknown;
        };
        let family = dialect.family();
        if !matches!(family, Some(Family::Tcl | Family::Jim)) {
            return Unknown;
        }
        let bindings = match self {
            Self::Update if count >= 4 && count.is_multiple_of(2) => {
                DictionaryScopeBindings::Pairs(
                    (offset + 1..offset + count - 1)
                        .step_by(2)
                        .map(|key| (key, key + 1))
                        .collect(),
                )
            }
            Self::With if count >= 2 => {
                DictionaryScopeBindings::AllKeys((offset + 1..offset + count - 1).collect())
            }
            _ => return Invalid,
        };
        let jim = family == Some(Family::Jim);
        let writeback = if jim && self == Self::With {
            DictionaryWriteback::NormalOnly
        } else {
            let CatchInvocationSelection::Valid(capture) = select_catch_invocation(
                InvocationArguments::literals(&[""]).with_dialect(dialect),
                dialect,
            ) else {
                return Unknown;
            };
            DictionaryWriteback::Captured(capture)
        };
        Selected(DictionaryScopePlan {
            implementation: if jim && self == Self::Update {
                DictionaryScopeImplementation::ScriptedWrapper
            } else {
                DictionaryScopeImplementation::NativePrimitive
            },
            dictionary_argument: offset,
            body_argument: offset + count - 1,
            bindings,
            missing_key: if jim {
                DictionaryMissingKey::Retain
            } else {
                DictionaryMissingKey::Unset
            },
            writeback,
            writeback_failure: if jim && self == Self::With {
                DictionaryWritebackFailure::NormalWithErrorResult
            } else {
                DictionaryWritebackFailure::Error
            },
            procedure_return_boundary: jim && self == Self::Update,
            dialect,
        })
    }
}

impl DictionaryScopePlan {
    /// Reached epilogue failure does not reuse a guessed wrapper return code.
    #[must_use]
    pub const fn writeback_failure_route(&self) -> InvocationCompletionRoute {
        InvocationCompletionRoute::Tcl(match self.writeback_failure {
            DictionaryWritebackFailure::Error => crate::CompletionCode::Error,
            DictionaryWritebackFailure::NormalWithErrorResult => crate::CompletionCode::Ok,
        })
    }

    /// Known completion paths reaching the epilogue. Unknown paths remain split.
    #[must_use]
    pub fn writeback_routes(
        &self,
        route: InvocationCompletionRoute,
    ) -> crate::catch_invocation::CatchCompletionRoutes {
        match self.writeback {
            DictionaryWriteback::Captured(capture) => capture.route(route),
            DictionaryWriteback::NormalOnly => crate::catch_invocation::CatchCompletionRoutes {
                captured: route.normal_possible(),
                propagated: route
                    .abrupt_possible()
                    .then_some(route)
                    .into_iter()
                    .collect(),
            },
        }
    }

    /// Restore the body's completion after successful writeback.
    /// Writeback errors take precedence and belong to the physical store owner.
    #[must_use]
    pub fn completion_after_writeback(
        &self,
        route: InvocationCompletionRoute,
    ) -> InvocationCompletionRoute {
        if self.procedure_return_boundary {
            route.through_procedure_boundary_in(Some(self.dialect))
        } else {
            route
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::completion::CompletionCode;
    use crate::completion_route::ReturnCompletionRoute;

    fn selected(spec: DictionaryScopeSpec, dialect: InvocationDialect) -> DictionaryScopePlan {
        let values: &[&str] = match spec {
            DictionaryScopeSpec::Update => &["d", "a", "v", "return BODY"],
            DictionaryScopeSpec::With => &["d", "return BODY"],
        };
        let DictionaryScopeSelection::Selected(plan) = spec.select(
            InvocationArguments::literals(values).with_dialect(dialect),
            0,
        ) else {
            panic!("known native scope must select")
        };
        plan
    }

    #[test]
    fn native_scope_completion_routes_match_dictionary_oracles() {
        let c = InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6);
        let jim = InvocationDialect::of_profile(
            crate::model::ingress::resolve_environment("jim").unit_profile(),
        );
        let returned = InvocationCompletionRoute::Return(ReturnCompletionRoute {
            eventual_code: CompletionCode::Ok,
            remaining_level: 1,
        });
        for spec in [DictionaryScopeSpec::Update, DictionaryScopeSpec::With] {
            let plan = selected(spec, c);
            assert!(plan.writeback_routes(returned).captured);
            assert_eq!(plan.completion_after_writeback(returned), returned);
            assert!(
                !plan
                    .writeback_routes(InvocationCompletionRoute::ProcessExit)
                    .captured
            );
        }
        let update = selected(DictionaryScopeSpec::Update, jim);
        assert_eq!(update.missing_key, DictionaryMissingKey::Retain);
        assert!(update.writeback_routes(returned).captured);
        assert_eq!(
            update.completion_after_writeback(returned),
            InvocationCompletionRoute::Tcl(CompletionCode::Ok)
        );
        let with = selected(DictionaryScopeSpec::With, jim);
        assert!(!with.writeback_routes(returned).captured);
        assert!(
            with.writeback_routes(InvocationCompletionRoute::Tcl(CompletionCode::Ok))
                .captured
        );
    }
}

/// Audited stock scripted implementation installed by the selected native distribution.
/// Command lookup remains mutable after bootstrap; consumers must not replace
/// calls to this procedure with a primitive based on its public name.
#[derive(Debug, Clone, Copy)]
pub struct ScriptedDictionaryWrapper {
    /// Tcl command name, including spaces when the distribution uses a multiword command.
    pub command: &'static str,
    /// Core dictionary selector whose native worker delegates to this procedure.
    pub subcommand: &'static str,
    /// Native formal parameter declaration.
    pub parameters: &'static str,
    /// Exact audited implementation script.
    pub body: &'static str,
    minimum_arguments: usize,
    maximum_arguments: Option<usize>,
    usage: &'static str,
}

const JIM_UPDATE: ScriptedDictionaryWrapper = ScriptedDictionaryWrapper {
    command: "dict update",
    subcommand: "update",
    parameters: "&varName args script",
    body: include_str!("native_scripts/jim_dict_update.tcl"),
    minimum_arguments: 2,
    maximum_arguments: None,
    usage: "varName ?arg ...? script",
};

const JIM_REPLACE: ScriptedDictionaryWrapper = ScriptedDictionaryWrapper {
    command: "dict replace",
    subcommand: "replace",
    parameters: "dictionary {args {key value}}",
    body: include_str!("native_scripts/jim_dict_replace.tcl"),
    minimum_arguments: 1,
    maximum_arguments: None,
    usage: "dictionary ?key value ...?",
};

const JIM_LAPPEND: ScriptedDictionaryWrapper = ScriptedDictionaryWrapper {
    command: "dict lappend",
    subcommand: "lappend",
    parameters: "varName key {args value}",
    body: include_str!("native_scripts/jim_dict_lappend.tcl"),
    minimum_arguments: 2,
    maximum_arguments: None,
    usage: "varName key ?value ...?",
};

const JIM_APPEND: ScriptedDictionaryWrapper = ScriptedDictionaryWrapper {
    command: "dict append",
    subcommand: "append",
    parameters: "varName key {args value}",
    body: include_str!("native_scripts/jim_dict_append.tcl"),
    minimum_arguments: 2,
    maximum_arguments: None,
    usage: "varName key ?value ...?",
};

const JIM_INCR: ScriptedDictionaryWrapper = ScriptedDictionaryWrapper {
    command: "dict incr",
    subcommand: "incr",
    parameters: "varName key {increment 1}",
    body: include_str!("native_scripts/jim_dict_incr.tcl"),
    minimum_arguments: 2,
    maximum_arguments: Some(3),
    usage: "varName key ?increment?",
};

const JIM_REMOVE: ScriptedDictionaryWrapper = ScriptedDictionaryWrapper {
    command: "dict remove",
    subcommand: "remove",
    parameters: "dictionary {args key}",
    body: include_str!("native_scripts/jim_dict_remove.tcl"),
    minimum_arguments: 1,
    maximum_arguments: None,
    usage: "dictionary ?key ...?",
};

const JIM_FOR: ScriptedDictionaryWrapper = ScriptedDictionaryWrapper {
    command: "dict for",
    subcommand: "for",
    parameters: "vars dictionary script",
    body: include_str!("native_scripts/jim_dict_for.tcl"),
    minimum_arguments: 3,
    maximum_arguments: Some(3),
    usage: "vars dictionary script",
};

impl DictionaryScopePlan {
    /// Authored scripted implementation, separate from proof it remains installed.
    #[must_use]
    pub const fn scripted_wrapper(&self) -> Option<&'static ScriptedDictionaryWrapper> {
        match self.implementation {
            DictionaryScopeImplementation::ScriptedWrapper => Some(&JIM_UPDATE),
            DictionaryScopeImplementation::NativePrimitive => None,
        }
    }
}

/// Wrappers bundled with the exact audited Jim distribution. This metadata
/// authorises native bootstrap only, never a source compiler stock-handler proof.
#[must_use]
pub fn stock_scripted_wrappers(dialect: InvocationDialect) -> &'static [ScriptedDictionaryWrapper] {
    if dialect
        .native_name_protocol()
        .is_some_and(tcl_syntax::naming::NativeNameProtocol::is_jim084)
    {
        &[
            JIM_UPDATE,
            JIM_REPLACE,
            JIM_LAPPEND,
            JIM_APPEND,
            JIM_INCR,
            JIM_REMOVE,
            JIM_FOR,
        ]
    } else {
        &[]
    }
}

/// Actual Jim dictionary core forwarding, independent of the wrapper's live binding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptedDictionaryDispatch {
    command: Vec<u8>,
    arguments: Option<usize>,
    usage: Vec<u8>,
}

impl ScriptedDictionaryDispatch {
    /// Original subcommand `CString` appended to the core's literal ensemble prefix.
    #[must_use]
    pub fn command(&self) -> &[u8] {
        &self.command
    }
    /// Number of original arguments forwarded; absence is core arity rejection.
    #[must_use]
    pub const fn arguments(&self) -> Option<usize> {
        self.arguments
    }
    /// Core usage for rejected argv, retaining the original selector spelling.
    #[must_use]
    pub fn usage(&self) -> &[u8] {
        &self.usage
    }
}

/// Select the native scripted worker schedule. A selected wrapper is metadata;
/// callers must resolve the generated command against the current command table.
#[must_use]
pub fn scripted_dictionary_dispatch(
    dialect: InvocationDialect,
    selected: &str,
    original: &[u8],
    count: usize,
) -> Option<ScriptedDictionaryDispatch> {
    let wrapper = stock_scripted_wrappers(dialect)
        .iter()
        .find(|wrapper| wrapper.subcommand == selected)?;
    // Jim_EvalEnsemble receives Jim_String(argv[1]) as a const char*.
    let original = &original[..original
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(original.len())];
    let mut command = b"dict ".to_vec();
    command.extend_from_slice(original);
    let mut usage = command.clone();
    usage.push(b' ');
    usage.extend_from_slice(wrapper.usage.as_bytes());
    let arguments = if count < wrapper.minimum_arguments
        || wrapper
            .maximum_arguments
            .is_some_and(|maximum| count > maximum)
    {
        None
    } else if selected == "update" && (count < 4 || !count.is_multiple_of(2)) {
        Some(0)
    } else {
        Some(count)
    };
    Some(ScriptedDictionaryDispatch {
        command,
        arguments,
        usage,
    })
}

#[cfg(test)]
mod scripted_tests {
    use super::*;

    fn jim() -> InvocationDialect {
        InvocationDialect::of_profile(
            crate::model::ingress::resolve_environment("jim").unit_profile(),
        )
    }

    #[test]
    fn scripted_workers_require_actual_jim_and_preserve_native_forwarding() {
        assert_eq!(stock_scripted_wrappers(jim()).len(), 7);
        for profile in ["tcl8.4", "tcl8.6", "tcl9.0", "f5"] {
            let profile = crate::model::ingress::resolve_environment(profile).unit_profile();
            assert!(stock_scripted_wrappers(InvocationDialect::of_profile(profile)).is_empty());
        }
        let abbreviated = scripted_dictionary_dispatch(jim(), "update", b"up", 4).unwrap();
        assert_eq!(abbreviated.command(), b"dict up");
        assert_eq!(abbreviated.arguments(), Some(4));
        let cstring = scripted_dictionary_dispatch(jim(), "update", b"update\0suffix", 4).unwrap();
        assert_eq!(cstring.command(), b"dict update");
        for count in [2, 3, 5] {
            assert_eq!(
                scripted_dictionary_dispatch(jim(), "update", b"update", count)
                    .unwrap()
                    .arguments(),
                Some(0)
            );
        }
        assert_eq!(
            scripted_dictionary_dispatch(jim(), "update", b"update", 1)
                .unwrap()
                .arguments(),
            None
        );
        assert_eq!(
            scripted_dictionary_dispatch(jim(), "incr", b"incr", 4)
                .unwrap()
                .arguments(),
            None
        );
        assert!(scripted_dictionary_dispatch(jim(), "with", b"with", 2).is_none());
    }
}
