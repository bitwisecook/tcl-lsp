//! `SIP::record-route` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "SIP::record-route",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "This command allows you get get information in the SIP record-route header.",
            synopsis: &["SIP::record-route (INDEX | 'top')"],
            snippet: "This command allows you get get information in the SIP record-route header.\n\nSynax\n\nSIP::record-route <index>\n\n     * Get SIP header \"route\" at index\n\n <index> is a numeric zero-based index or the keyword \"top\". The \"top\" keyword\" will acess the first element as opposed to the first line of the record-route headers.",
            source: "https://clouddocs.f5.com/api/irules/SIP__record_route.html",
            examples: "when SIP_REQUEST {\n  log local0. [SIP::recordroute top]\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["SIP"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "SIP::record-route (INDEX | 'top')" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::NetworkIo,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
