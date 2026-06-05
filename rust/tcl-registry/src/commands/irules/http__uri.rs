//! `HTTP::uri` iRules command.
use crate::prelude::*;
use crate::taint::SetterConstraint;

/// GAP-D2: the setter form of `HTTP::uri` requires its value to start
/// with `/` (IRULE3101). Registry-driven replacement for the hardcoded
/// `SETTER_CONSTRAINTS` table in `tcl_compiler::taint`. Mirrors
/// `irules/http__uri.py`.
const SETTER_CONSTRAINTS: &[SetterConstraint] = &[SetterConstraint {
    arg_index: 0,
    required_prefix: "/",
    code: "IRULE3101",
    message: "HTTP::uri value must start with '/'",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP::uri",
        traits: Traits::PURE | Traits::CSE_CANDIDATE | Traits::DIAGRAM_ACTION,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::new(0, 1),
        options: &[OptionSpec {
            name: "-normalized",
            takes_value: false,
            value_hint: "",
            detail: "Return the canonicalised URI (URL evasion patterns rejected).",
            dialects: None,
        }],
        hover: Some(HoverSnippet::brief(
            "Returns or sets the URI part of the HTTP request.",
            &["HTTP::uri (URI)?"],
            "F5 iRules",
        )),
        setter_constraints: SETTER_CONSTRAINTS,
        ..CommandSpec::DEFAULT
    }
}
