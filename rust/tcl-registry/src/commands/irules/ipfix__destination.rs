//! `IPFIX::destination` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "IPFIX::destination",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "IPFIX::destination Provides the ability to manage IPFIX logging destinations and send IPFIX messages based on processing in the iRule.",
            synopsis: &["IPFIX::destination ((open (-publisher LOG_PUBLISHER)) |"],
            snippet: "Provides the ability to open and close IPFIX logging destinations in\nthe context of an iRule, as well as the ability to send IPFIX messages\nto the IPFIX logging destinations.",
            source: "https://clouddocs.f5.com/api/irules/IPFIX__destination.html",
            examples: "when RULE_INIT {\n    set static::http_track_dest \"\"\n    set static::http_track_tmplt \"\"\n}",
            return_value: "IPFIX::destination open returns an IPFIX_DESTINATION object that is used by the IPFIX::destination close or send command.",
        }),
        ..CommandSpec::DEFAULT
    }
}
