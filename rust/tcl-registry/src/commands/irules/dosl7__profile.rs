//! `DOSL7::profile` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "DOSL7::profile",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the DOS profile from which the L7-DoS policy is extracted.",
            synopsis: &["DOSL7::profile"],
            snippet: "This command returns the DOS profile from which the L7-DoS policy is\nextracted.\nNote:\n  * in 11.4, default policy returns empty string and if L7-DoS is\n    disabled, the <no-profile> string is returned.\n  * in 11.5+, default policy returns the one configured with the vip\n    and if L7-DoS is disabled, a null string is returned.",
            source: "https://clouddocs.f5.com/api/irules/DOSL7__profile.html",
            examples: "",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["FASTHTTP"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "DOSL7::profile" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::Dosl7State,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Global,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
