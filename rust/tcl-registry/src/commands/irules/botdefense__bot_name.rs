//! `BOTDEFENSE::bot_name` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "BOTDEFENSE::bot_name",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the name assigned to the detected bot, browser or mobile application.",
            synopsis: &["BOTDEFENSE::bot_name"],
            snippet: "Returns the name assigned to the detected bot, browser or mobile application. The name is derived from the detected signature if detected, or the User-Agent string in combination with the detected anomalies.",
            source: "https://clouddocs.f5.com/api/irules/BOTDEFENSE__bot_name.html",
            examples: "# EXAMPLE: Log the Bot name and Device ID of the client, upon each request, if it is known.\nwhen BOTDEFENSE_ACTION {\n    log local0.info \"Bot [BOTDEFENSE::bot_name] with Device ID [ BOTDEFENSE::device_id] from IP [ IP::client_addr ] visited [HTTP::uri ]\"\n}",
            return_value: "The name assigned to the bot, browser or mobile application that sent the request.",
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
            FormSpec { kind: FormKind::Default, synopsis: "BOTDEFENSE::bot_name" },
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
