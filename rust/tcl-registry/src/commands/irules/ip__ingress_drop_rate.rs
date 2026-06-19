//! `IP::ingress_drop_rate` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "IP::ingress_drop_rate",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Adds ip with specified drop rate to black list table.",
            synopsis: &["IP::ingress_drop_rate"],
            snippet: "This command adds ip with specified drop rate to black list table, table enforced per packet containing source ip for specified timeout.",
            source: "https://clouddocs.f5.com/api/irules/IP__ingress_drop_rate.html",
            examples: "",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: true,
            server_side: false,
            transport: None,
            profiles: &[],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "IP::ingress_drop_rate",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::ConnectionControl,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
        }],
        ..CommandSpec::DEFAULT
    }
}
