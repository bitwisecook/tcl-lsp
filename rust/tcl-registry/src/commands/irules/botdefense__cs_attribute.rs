//! `BOTDEFENSE::cs_attribute` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "BOTDEFENSE::cs_attribute",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Queries for or sets attributes for the client-side challenge.",
            synopsis: &["BOTDEFENSE::cs_attribute 'device_id' (BOOLEAN)?"],
            snippet: "Queries for or sets attributes for the client-side challenge. These attributes are only effective if a client-side action is taken on the current request.",
            source: "https://clouddocs.f5.com/api/irules/BOTDEFENSE__cs_attribute.html",
            examples: "# EXAMPLE: Make sure that the data for the device_id is always collected when taking a client-side action.\nwhen BOTDEFENSE_REQUEST {\n    BOTDEFENSE::cs_attribute device_id enable\n}",
            return_value: "* When called with an argument the command overrides the decision of Bot Defense whether to collect device id. * When called without an argument, the command returns whether Bot Defense attempts to collect the device id during the request (initiate response).",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["BOTDEFENSE"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "BOTDEFENSE::cs_attribute 'device_id' (BOOLEAN)?" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::AsmState,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Client,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
