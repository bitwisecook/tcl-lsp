//! `BOTDEFENSE::bot_signature` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "BOTDEFENSE::bot_signature",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the name of the detected Bot Signature.",
            synopsis: &["BOTDEFENSE::bot_signature"],
            snippet: "Returns the name of the detected Bot Signature, or an empty string if no bot signature was detected.",
            source: "https://clouddocs.f5.com/api/irules/BOTDEFENSE__bot_signature.html",
            examples: "# EXAMPLE: Log the bot signature.\nwhen BOTDEFENSE_REQUEST {\n    set log \"botdefense bot_signature is\"\n    append log \" [BOTDEFENSE::bot_signature]\"\n    HSL::send $hsl $log\n}",
            return_value: "Returns the name of the detected Bot Signature, or an empty string if no bot signature was detected.",
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
            FormSpec { kind: FormKind::Default, synopsis: "BOTDEFENSE::bot_signature" },
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
