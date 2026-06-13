//! `NSH::path_id` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "NSH::path_id",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Set/Get the Path ID for NSH.",
            synopsis: &["NSH::path_id DIRECTION (NSH_PATH_ID)?"],
            snippet: "Set: Path ID for NSH.\n            Get(DIRECTION as the only parameter): path id from NSH.",
            source: "https://clouddocs.f5.com/api/irules/NSH__path_id.html",
            examples: "th ID for NSH.\n            when CLIENT_ACCEPTED {\n                NSH::path_id serverside_egress 10\n                set mypath_id [NSH::path_id serverside_egress]\n            }",
            return_value: "None for set, value of path id for get.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "NSH::path_id DIRECTION (NSH_PATH_ID)?" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::NetworkIo,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
