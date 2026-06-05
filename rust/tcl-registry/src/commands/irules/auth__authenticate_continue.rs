//! `AUTH::authenticate_continue` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "AUTH::authenticate_continue",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Continues an authentication operation.",
            synopsis: &["AUTH::authenticate_continue AUTH_ID RESPONSE"],
            snippet: "Continues an authentication operation by providing the specified string\nas the credential response for the most recent authorization prompt.\nThis command is only available when the event AUTH_WANTCREDENTIAL is\nthe most recent event generated, and no AUTH::credential commands have\nbeen issued since the event, for the specified authentication ID.\nUnlike the AUTH::credential commands, the string credential provided by\nthis command does not get cached, even if the desired credential type\nhad been identified (see the AUTH::wantcredential_type command).",
            source: "https://clouddocs.f5.com/api/irules/AUTH__authenticate_continue.html",
            examples: "when CLIENT_ACCEPTED {\n    set auth_stage 0\n}",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "AUTH::authenticate_continue AUTH_ID RESPONSE" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::ApmState,
                reads: false,
                writes: true,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
