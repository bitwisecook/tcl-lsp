//! `SSL::profile` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "SSL::profile",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Switch between different SSL profiles.",
            synopsis: &["SSL::profile PROFILE_OBJ"],
            snippet: "This command allows you to switch between SSL profiles, both client and server. Note: This should be done before the SSL negotiation occurs, or your rule will require the use of the SSL::renegotiate command.\n\nIn order to switch SSL profiles, a profile must be assigned to the virtual to begin with; switching the clientssl profile requires an existing clientssl profile, and similarly for serverssl profiles. You can also use SSL::disable to use SSL selectively.",
            source: "https://clouddocs.f5.com/api/irules/SSL__profile.html",
            examples: "when HTTP_REQUEST {\n    SSL::renegotiate\n}",
            return_value: "SSL::profile <profile_name> Switch to the defined SSL profile.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "SSL::profile PROFILE_OBJ" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::SslState,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
