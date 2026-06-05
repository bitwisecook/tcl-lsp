//! `BIGPROTO::enable_fix_reset` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "BIGPROTO::enable_fix_reset",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Enable or Disable Reset of FIX Protocol Connections",
            synopsis: &["BIGPROTO::enable_fix_reset BOOLEAN"],
            snippet: "When set to disabled, TCP RST frame will not be sent when BIG-IP detects there is a hash collision on ePVA offloading of FIX flows. Instead, it will try to re-offload the connection.",
            source: "https://clouddocs.f5.com/api/irules/BIGPROTO__enable_fix_reset.html",
            examples: "when CLIENT_ACCEPTED {\n    BIGPROTO::enable_fix_reset true\n    BIGPROTO::enable_fix_reset false\n            }",
            return_value: "none",
        }),
        ..CommandSpec::DEFAULT
    }
}
