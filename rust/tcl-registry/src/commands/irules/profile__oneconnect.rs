//! `PROFILE::oneconnect` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "PROFILE::oneconnect",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the value of a Oneconnect profile setting.",
            synopsis: &["PROFILE::oneconnect ATTR"],
            snippet: "Returns the current value of the specified setting in the assigned Oneconnect profile.",
            source: "https://clouddocs.f5.com/api/irules/PROFILE__oneconnect.html",
            examples: "",
            return_value: "Returns the current value of the specified setting in the assigned Oneconnect profile.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "PROFILE::oneconnect ATTR" },
        ],
        ..CommandSpec::DEFAULT
    }
}
