//! `NSH::service_index` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "NSH::service_index",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Sets/Get the Service Index for NSH.",
            synopsis: &["NSH::service_index DIRECTION (NSH_SERVICE_IDX)?"],
            snippet: "Set: Service index for NSH.\n            Get(DIRECTION as the only parameter): Service index from NSH.",
            source: "https://clouddocs.f5.com/api/irules/NSH__service_index.html",
            examples: "rvice index for NSH.\n            when CLIENT_ACCEPTED {\n                NSH::service_index serverside_egress 20\n                set myservice_index [NSH::service_index serverside_egress]\n            }",
            return_value: "None.",
        }),
        ..CommandSpec::DEFAULT
    }
}
