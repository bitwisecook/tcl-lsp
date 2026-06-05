//! `SDP::media` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SDP::media",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Get or set SDP media information.",
            synopsis: &["SDP::media (count | MEDIA_INDEX)?", "SDP::media (type | transport) (MEDIA_INDEX)?", "SDP::media attr (MEDIA_INDEX (ATTR_INDEX)?)?", "SDP::media port (MEDIA_INDEX (NEW_PORT)?)?"],
            snippet: "This command allows you to get or set different aspects of the media\ninformation for your SDP connection.",
            source: "https://clouddocs.f5.com/api/irules/SDP__media.html",
            examples: "when SIP_REQUEST {\n    log local0. \"SDP media count: [SDP::media count]\"\n    log local0. \"SDP media transport: [SDP::media transport 0]\"\n    log local0. \"SDP media port: [SDP::media port 0]\"\n    log local0. \"SDP media connection: [SDP::media conn 0]\"\n}",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}
