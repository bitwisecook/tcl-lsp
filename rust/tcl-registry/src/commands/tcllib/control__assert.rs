//! `control::assert` command.
use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "control::assert expr ?arg arg ...?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "control::assert",
        dialects: None,
        arity: Arity::at_least(1),
        // `expr` is evaluated as an expression, like `expr` itself.
        arg_roles: &[(0, ArgRole::Expr)],
        arg_types: &[(
            0,
            ArgTypeHint {
                expected: Some(TclType::Boolean),
                shimmers: true,
            },
        )],
        hover: Some(HoverSnippet {
            summary: "Raise an error if a boolean expression is false (when enabled).",
            synopsis: &["control::assert expr ?arg arg ...?"],
            snippet: "When enabled, evaluates expr as a boolean expression and raises an error if it is false. Any trailing arguments form the failure message. When disabled, behaves like control::no-op.",
            source: "tcllib control package",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        tcllib_package: Some("control"),
        required_package: Some("control"),
        ..CommandSpec::DEFAULT
    }
}
