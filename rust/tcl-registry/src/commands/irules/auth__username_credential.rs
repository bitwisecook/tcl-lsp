//! `AUTH::username_credential` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "AUTH::username_credential",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Sets the username credential to a string.",
            synopsis: &["AUTH::username_credential AUTH_ID USERNAME_CREDENTIAL"],
            snippet: "Sets the username credential to the specified string, for a future\nAUTH::authenticate call. This command returns an error if\nattempted for a standby system.\n\nAUTH::username_credential authid <string>\n\n     * Sets the username credential to the specified string, for a future\n       AUTH::authenticate call.",
            source: "https://clouddocs.f5.com/api/irules/AUTH__username_credential.html",
            examples: "when HTTP_REQUEST {\n  AUTH::username_credential $asid [HTTP::username]\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "AUTH::username_credential AUTH_ID USERNAME_CREDENTIAL",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::ApmState,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Both,
        }],
        ..CommandSpec::DEFAULT
    }
}
