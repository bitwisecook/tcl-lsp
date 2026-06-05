//! `PROFILE::exists` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "PROFILE::exists",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Determine if a profile is configured on a virtual server.",
            synopsis: &["PROFILE::exists TYPE (NAME)?", "PROFILE::exists persist MODE (NAME)?"],
            snippet: "Determine if a profile is configured on a virtual server.\n\nNote that the results of the PROFILE::exists \"profile type\" command is specific to the context of the event. For example, with a client SSL profile associated with the virtual server, PROFILE::exists clientssl will return 1 in clientside events and 0 in serverside events. Likewise, PROFILE::exists serverssl will return 0 in clientside events and 1 in serverside events.",
            source: "https://clouddocs.f5.com/api/irules/PROFILE__exists.html",
            examples: "when CLIENT_ACCEPTED {\n   if { [PROFILE::exists clientssl] == 1} {\n      log local0. \"client SSL profile enabled on virtual server\"\n   }\n}",
            return_value: "Returns 1 if the profile is configured on the current virtual server. Returns 0 if the profile is not configured on the current virtual server.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "PROFILE::exists TYPE (NAME)?" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::BigipConfig,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Global,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
