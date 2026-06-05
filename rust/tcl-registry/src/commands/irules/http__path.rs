//! `HTTP::path` iRules command.
use crate::prelude::*;
use crate::taint::SetterConstraint;

/// GAP-D2: the setter form of `HTTP::path` requires its value to start
/// with `/` (IRULE3101). Registry-driven replacement for the hardcoded
/// `SETTER_CONSTRAINTS` table in `tcl_compiler::taint`. Mirrors
/// `irules/http__path.py`.
const SETTER_CONSTRAINTS: &[SetterConstraint] = &[SetterConstraint {
    arg_index: 0,
    required_prefix: "/",
    code: "IRULE3101",
    message: "HTTP::path value must start with '/'",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP::path",
        traits: Traits::PURE | Traits::CSE_CANDIDATE | Traits::DIAGRAM_ACTION,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        options: &[OptionSpec {
            name: "-normalized",
            takes_value: false,
            value_hint: "",
            detail: "Return the canonicalised path (URL evasion patterns rejected).",
            dialects: None,
        }],
        hover: Some(HoverSnippet::brief(
            "Returns or sets the path part of the HTTP request.",
            &["HTTP::path (PATH_VALUE)?"],
            "F5 iRules",
        )),
        setter_constraints: SETTER_CONSTRAINTS,
        ..CommandSpec::DEFAULT
    }
}
