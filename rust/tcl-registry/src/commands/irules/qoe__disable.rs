//! `QOE::disable` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "QOE::disable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Deprecated: Disables the video QOE filter from processing any video or non-video traffic on a connection basis.",
            synopsis: &["QOE::disable"],
            snippet: "This command disables the video QOE filter from processing any video or non-video traffic on a connection basis.",
            source: "https://clouddocs.f5.com/api/irules/QOE__disable.html",
            examples: "",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["CLASSIFICATION", "FASTHTTP", "QOE"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "QOE::disable" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::ConnectionControl,
                reads: false,
                writes: true,
                connection_side: ConnectionSide::Both,
            },
        ],
        deprecated_replacement: Some("(removed)"),
        ..CommandSpec::DEFAULT
    }
}
