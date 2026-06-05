//! `SSL::sni` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SSL::sni",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns Server Name Indication information.",
            synopsis: &["SSL::sni (name | required)"],
            snippet: "Returns a Server Name Indication name, and require SNI support.",
            source: "https://clouddocs.f5.com/api/irules/SSL__sni.html",
            examples: "when HTTP_REQUEST {\n    log local0.info \"SNI name: [SSL::sni name]\"\n    log local0.info \"SNI required: [SSL::sni required]\"\n}",
            return_value: "SSL::sni name Returns the current Server Name Indication as specified in the SSL profile. SSL::sni required Returns the require SNI support as specified in the SSL profile.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "SSL::sni <name | required>" },
        ],
        ..CommandSpec::DEFAULT
    }
}
