//! `VALIDATE::protocol` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "VALIDATE::protocol",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Performs validation of given application to match payload.",
            synopsis: &["VALIDATE::protocol CLASSIFY_APP_NAME ANY_CHARS"],
            snippet: "This command allows you to validate payload (traffic) to match given classification application.\n\nNote: the APM / AFM / PEM license is required for functionality to work.",
            source: "https://clouddocs.f5.com/api/irules/VALIDATE__protocol.html",
            examples: "when CLIENT_ACCEPTED {\n  TCP::collect 32\n}",
            return_value: "Returns TRUE in case of match, FALSE otherwise.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "VALIDATE::protocol CLASSIFY_APP_NAME ANY_CHARS" },
        ],
        ..CommandSpec::DEFAULT
    }
}
