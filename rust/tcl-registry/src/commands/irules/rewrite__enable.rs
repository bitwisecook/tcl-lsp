//! `REWRITE::enable` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "REWRITE::enable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Changes the REWRITE plugin from passthrough to full patching mode.",
            synopsis: &["REWRITE::enable"],
            snippet: "Changes the REWRITE plugin from passthrough to full patching mode. A\nplace where this might be helpful would be a POST request where REWRITE\nwould modify the post body unnecessarily, so we disable it. However, we\nwant REWRITE to modify the response, so we would enable it later in the\nHTTP_RESPONSE. Use of this command can be extremely tricky to get\nexactly right; its use is not recommended in the majority of cases",
            source: "https://clouddocs.f5.com/api/irules/REWRITE__enable.html",
            examples: "",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["ACCESS", "FASTHTTP", "REWRITE"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "REWRITE::enable",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::StreamProfile,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Both,
        }],
        ..CommandSpec::DEFAULT
    }
}
