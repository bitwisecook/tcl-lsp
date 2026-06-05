//! `DNSMSG::section` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DNSMSG::section",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns a section of a dns_message.",
            synopsis: &["DNSMSG::section DNS_MESSAGE ('question' | 'answer' | 'authority' | 'additional' )"],
            snippet: "This iRule gets the specified section of a dns_message.",
            source: "https://clouddocs.f5.com/api/irules/DNSMSG-section.html",
            examples: "when CLIENT_ACCEPTED {\n        set result [RESOLVER::name_lookup \"/Common/r1\" www.abc.com a]\n        set answer [DNSMSG::section $result answer]\n}",
            return_value: "Returns a TCL list of resource records from the specified section.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "DNSMSG::section DNS_MESSAGE ('question' | 'answer' | 'authority' | 'additional' )" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::DnsState,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
