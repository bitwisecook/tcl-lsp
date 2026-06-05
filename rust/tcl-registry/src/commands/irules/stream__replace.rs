//! `STREAM::replace` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "STREAM::replace",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Changes a replacement string in the Stream profile.",
            synopsis: &["STREAM::replace (TARGET_STRING)?"],
            snippet: "Changes the specified target replacement string in the Stream profile.\nThis command is not sticky and is applied only once during the current\nmatch. If the target expression is missing, the replacement is skipped.",
            source: "https://clouddocs.f5.com/api/irules/STREAM__replace.html",
            examples: "when STREAM_MATCHED {\n    set server [string tolower [STREAM::match]]\n    if {$server contains \"mail\"} {\n        STREAM::replace \"webmail.yourdomain.com/$mailhost\"\n    }\n}",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "STREAM::replace (TARGET_STRING)?" },
        ],
        ..CommandSpec::DEFAULT
    }
}
