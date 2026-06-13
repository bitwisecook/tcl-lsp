//! `ACCESS::flowid` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ACCESS::flowid",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Sets the flow id for SSL Orchestrator using APM logging framework.",
            synopsis: &["ACCESS::flowid (FID)?"],
            snippet: "ACCESS::flowid [FID]\n\nCalculates the flow id from the IFC and 4-tuple information, if it doesn't\nexist already, and stores it in the opaque storage for the connflow.\nRequires APM to be provisioned.\n\nCommand Syntax\n\nACCESS::flowid\n\n    * Returns the flow id, if it exists, or calculates it, then stores it in\n      the opaque data structure for the connflow.\n\nACCESS::flowid <FID>\n\n    * Sets the flow id to FID",
            source: "https://clouddocs.f5.com/api/irules/ACCESS__flowid.html",
            examples: "when HTTP_REQUEST {\n    ACCESS::flowid \"example\"\n    set ctx(FID) [ACCESS::flowid]\n}",
            return_value: "The flow id is returned",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "ACCESS::flowid (FID)?" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::ApmState,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
