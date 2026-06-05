//! `LB::mode` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "LB::mode",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Sets the load balancing mode, overriding the mode set in the pool definition.",
            synopsis: &["LB::mode (default | rr | roundrobin)", "LB::mode (leastconns | nodeleastconns)", "LB::mode (fastest)", "LB::mode (predictive)"],
            snippet: "Sets the load balancing mode, overriding the mode set in the pool definition\n\nLB::mode [default | rr | roundrobin | leastconns |\n          fastest | predictive | observed | ratio |\n          dynratio | nodeleastconns | noderatio]",
            source: "https://clouddocs.f5.com/api/irules/LB__mode.html",
            examples: "when LB_SELECTED {\n    if { $myretry >= 1 } {\n        LB::mode rr\n        LB::reselect pool $mypool\n    }\n}",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "LB::mode <mode>" },
        ],
        ..CommandSpec::DEFAULT
    }
}
