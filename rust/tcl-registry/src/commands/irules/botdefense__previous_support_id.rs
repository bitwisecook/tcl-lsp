//! `BOTDEFENSE::previous_support_id` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "BOTDEFENSE::previous_support_id",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the Device ID of the client, as retrieved from the request.",
            synopsis: &["BOTDEFENSE::previous_support_id"],
            snippet: "Returns the Support ID of the previous request; this is applicable if the current request is a follow-up to a challenge. Otherwise, \"0\" is returned.",
            source: "https://clouddocs.f5.com/api/irules/BOTDEFENSE__previous_support_id.html",
            examples: "# EXAMPLE: Log the Support ID of the previous request.\nwhen BOTDEFENSE_REQUEST {\n    set log \"botdefense previous support ID is\"\n    append log \" [BOTDEFENSE::previous_support_id]\"\n    HSL::send $hsl $log\n}",
            return_value: "Returns the support ID of the previous request, or 0 if not applicable.",
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
            FormSpec { kind: FormKind::Default, synopsis: "BOTDEFENSE::previous_support_id" },
        ],
        ..CommandSpec::DEFAULT
    }
}
