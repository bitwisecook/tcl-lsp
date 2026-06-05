//! `DNS::last_act` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DNS::last_act",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Sets the action to perform if no DNS service handles this packet.",
            synopsis: &["DNS::last_act ('allow' | 'drop' | 'reject' | 'hint' | 'noerror')"],
            snippet: "This iRules command sets the action to perform if no DNS service\nhandles this packet\n\nNote: This command requires the DNS Profile, which is only enabled as\npart of GTM or the DNS Services add-on.",
            source: "https://clouddocs.f5.com/api/irules/DNS__last_act.html",
            examples: "equests that are not handled by a local dns service\n            when DNS_REQUEST {\n                DNS::last_act drop\n            }",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["DNS"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "DNS::last_act ('allow' | 'drop' | 'reject' | 'hint' | 'noerror')" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::DnsState,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
