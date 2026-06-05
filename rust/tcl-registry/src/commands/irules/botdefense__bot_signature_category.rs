//! `BOTDEFENSE::bot_signature_category` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "BOTDEFENSE::bot_signature_category",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the name of the detected Bot Signature Category.",
            synopsis: &["BOTDEFENSE::bot_signature_category"],
            snippet: "Returns the name of the detected Bot Signature Category, or an empty string if no bot signature was detected.",
            source: "https://clouddocs.f5.com/api/irules/BOTDEFENSE__bot_signature_category.html",
            examples: "# EXAMPLE: Log the bot signature category.\nwhen BOTDEFENSE_REQUEST {\n    set log \"botdefense bot_signature_category is\"\n    append log \" [BOTDEFENSE::bot_signature_category]\"\n    HSL::send $hsl $log\n}",
            return_value: "Returns the name of the detected Bot Signature Category, or an empty string if no bot signature was detected.",
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
            FormSpec { kind: FormKind::Default, synopsis: "BOTDEFENSE::bot_signature_category" },
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
