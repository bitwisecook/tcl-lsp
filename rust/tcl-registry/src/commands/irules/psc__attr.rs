//! `PSC::attr` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "PSC::attr",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Gets, sets or removes the custom attributes.",
            synopsis: &[
                "PSC::attr ((NAME) | (NAME VALUE))?",
                "PSC::attr 'remove' (NAME)?",
            ],
            snippet: "The PSC::attr commands get/set/remove the custom attributes.",
            source: "https://clouddocs.f5.com/api/irules/PSC__attr.html",
            examples: "",
            return_value:
                "* PSC::attr Return the list of custom attribute names when no argument is given.",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "PSC::attr ((NAME) | (NAME VALUE))?",
        }],
        ..CommandSpec::DEFAULT
    }
}
