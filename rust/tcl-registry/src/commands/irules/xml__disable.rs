//! `XML::disable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "XML::disable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Changes the XML plugin from full patching mode to passthrough.",
            synopsis: &["XML::disable"],
            snippet: "Changes the XML plugin from full patching mode to passthrough.",
            source: "https://clouddocs.f5.com/api/irules/XML__disable.html",
            examples: "",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "XML::disable",
        }],
        ..CommandSpec::DEFAULT
    }
}
