//! `GTP::forward` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "GTP::forward",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Forwards GTP message to peer flow.",
            synopsis: &["GTP::forward MESSAGE"],
            snippet: "Forwards GTP message to peer flow.",
            source: "https://clouddocs.f5.com/api/irules/GTP__forward.html",
            examples:
                "when GTP_SIGNALLING_INGRESS {\n    set t2 [GTP::new 2 10]\n    GTP::forward $t2\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "GTP::forward MESSAGE",
        }],
        ..CommandSpec::DEFAULT
    }
}
