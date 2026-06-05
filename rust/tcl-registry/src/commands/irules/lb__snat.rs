//! `LB::snat` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "LB::snat",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns information on the SNAT configuration for the current connection.",
            synopsis: &["LB::snat"],
            snippet: "This command returns information on the SNAT configuration for the current connection.\n\nPossible output values are those which can be set by the snat and snatpool commands.",
            source: "https://clouddocs.f5.com/api/irules/LB__snat.html",
            examples: "when CLIENT_ACCEPTED {\n    # Check if SNAT is enabled on the VIP\n    if {[LB::snat] eq \"none\"}{\n        log local0. \"Snat disabled on [virtual name]\"\n    } else {\n        log local0. \"Snat enabled on [virtual name].  Currently set to [LB::snat]\"\n    }\n}",
            return_value: "LB::snat",
        }),
        ..CommandSpec::DEFAULT
    }
}
