//! `findstr` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "findstr",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Finds a string within another string and returns the string starting at the offset specified from the match.",
            synopsis: &["findstr STRING SEARCH_STRING ("],
            snippet: "A custom iRule function which finds a string within another string\nand returns the string starting at the offset specified from the match.",
            source: "https://clouddocs.f5.com/api/irules/findstr.html",
            examples: "when RULE_INIT {\n  set static::payload {<meta HTTP-EQUIV=\"REFRESH\" CONTENT=\"0; URL=https://host.domain.com/path/file.ext?...&var=val\">}\n  set static::term {\">}\n  set urlresponse [findstr $static::payload URL= 4 $static::term]\n  log local0. \"urlresponse $urlresponse\"\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "findstr STRING SEARCH_STRING (",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::Unknown,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Global,
        }],
        ..CommandSpec::DEFAULT
    }
}
