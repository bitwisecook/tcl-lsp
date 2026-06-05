//! `PSC::subscriber_id` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "PSC::subscriber_id",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Get or set the subscriber id.",
            synopsis: &["PSC::subscriber_id ((SUBSCRIBER_ID) (e164 |"],
            snippet: "The PSC::subscriber_id command gets the subscriber id or sets the subscriber_id when the optional value is given.",
            source: "https://clouddocs.f5.com/api/irules/PSC-subscriber-id.html",
            examples: "",
            return_value: "Return the subscriber id when no argument is given.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "PSC::subscriber_id ((SUBSCRIBER_ID) (e164 |" },
        ],
        ..CommandSpec::DEFAULT
    }
}
