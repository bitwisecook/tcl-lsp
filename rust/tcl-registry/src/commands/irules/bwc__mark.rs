//! `BWC::mark` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "BWC::mark",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "This command allows you to set or unset marking for traffic flows in bwc when configured rate limit is exceeded.",
            synopsis: &["BWC::mark SESSION_ID ('qos'|'tos') (BWC_VALUE | 'passthrough')", "BWC::mark SESSION_ID APP_NAME ('qos'|'tos') (BWC_VALUE | 'passthrough')"],
            snippet: "This command allows you to set or unset marking for traffic flows in bwc when configured rate limit is exceeded. Marking can be on DSCP (ToS - L3) and/or QoS (L2). The ToS/QoS value needs to be in valid range and can be passthrough.",
            source: "https://clouddocs.f5.com/api/irules/BWC__mark.html",
            examples: "when CLIENT_ACCEPTED {\n    set mycookie [IP::remote_addr]:[TCP::remote_port]\n    BWC::policy attach gold_user $mycookie\n    BWC::color set gold_user p2p\n}",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "BWC::mark SESSION_ID ('qos'|'tos') (BWC_VALUE | 'passthrough')" },
        ],
        ..CommandSpec::DEFAULT
    }
}
