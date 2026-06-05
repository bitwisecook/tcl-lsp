//! `PROFILE::xml` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "PROFILE::xml",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the value of an XML profile setting.",
            synopsis: &["PROFILE::xml ATTR"],
            snippet:
                "Returns the current value of the specified setting in an assigned XML profile.",
            source: "https://clouddocs.f5.com/api/irules/PROFILE__xml.html",
            examples: "",
            return_value:
                "Returns the current value of the specified setting in an assigned XML profile.",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "PROFILE::xml ATTR",
        }],
        ..CommandSpec::DEFAULT
    }
}
