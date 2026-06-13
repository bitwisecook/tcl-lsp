//! `STREAM::encoding` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "STREAM::encoding",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Specifies non-default content encoding.",
            synopsis: &["STREAM::encoding (ascii | utf-8 | unicode)"],
            snippet: "Specifies non-default content encoding. The default value is ascii.",
            source: "https://clouddocs.f5.com/api/irules/STREAM__encoding.html",
            examples: "when STREAM_MATCHED {\n    set stream_match [STREAM::match]\n    log local0. \"$stream_match\"\n    STREAM::encoding utf-8\n    # The ?/? represents unicode characters.\n    if { $stream_match contains \"hello?/?\" } {\n        STREAM::replace \"hello hey\"\n        log local0. \"stream match is [STREAM::match]\"\n    }\n}",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "STREAM::encoding (ascii | utf-8 | unicode)" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::StreamProfile,
                reads: false,
                writes: true,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
