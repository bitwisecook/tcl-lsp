//! `incr` — increment a variable.

use crate::forms::CommandForm;
use crate::hooks::LoweringHookId;
use crate::prelude::*;

/// `incr varName` — implicit increment of 1.
const INCR_IMPLICIT: CommandForm = CommandForm {
    name: "implicit",
    arity: Arity::exact(1),
    arg_roles: &[(0, ArgRole::VarWrite)],
    lowering_hook: Some(LoweringHookId::Incr),
    ..CommandForm::DEFAULT
};

/// `incr varName increment` — explicit increment.
const INCR_EXPLICIT: CommandForm = CommandForm {
    name: "explicit",
    arity: Arity::exact(2),
    arg_roles: &[(0, ArgRole::VarWrite)],
    lowering_hook: Some(LoweringHookId::Incr),
    ..CommandForm::DEFAULT
};

/// Command spec for `incr`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "incr",
        traits: Traits::READS_BEFORE_WRITE,
        arity: Arity::new(1, 2),
        arg_roles: &[(0, ArgRole::VarWrite)],
        assigns_variable_at: Some(0),
        safe_on_uninit: Some(DialectSet::TCL85_PLUS),
        return_type: Some(TclType::Int),
        arg_types: &[
            (
                0,
                ArgTypeHint {
                    expected: Some(TclType::Int),
                    shimmers: true,
                },
            ),
            (
                1,
                ArgTypeHint {
                    expected: Some(TclType::Int),
                    shimmers: true,
                },
            ),
        ],
        hover: Some(HoverSnippet::brief(
            "Increment the value of a variable.",
            &["incr varName ?increment?"],
            "Tcl incr(1)",
        )),
        lowering_hook: Some(LoweringHookId::Incr),
        command_forms: &[INCR_IMPLICIT, INCR_EXPLICIT],
        ..CommandSpec::DEFAULT
    }
}
