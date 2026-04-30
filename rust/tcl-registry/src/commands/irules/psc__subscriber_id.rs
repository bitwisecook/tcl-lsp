//! `PSC::subscriber_id` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "PSC::subscriber_id",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Get or set the subscriber id.",
            &["PSC::subscriber_id ((SUBSCRIBER_ID) (e164 |"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
