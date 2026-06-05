//! `ip_ttl` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ip_ttl",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Synonym for IP::ttl. Returns the TTL of the latest IP packet received.",
            synopsis: &["ip_ttl"],
            snippet: "Synonym for IP::ttl. Returns the TTL of the latest IP packet\nreceived.",
            source: "https://clouddocs.f5.com/api/irules/ip_ttl.html",
            examples: "when CLIENT_ACCEPTED {\n  log local0. \"Client ttl: [ip_ttl]\"\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "ip_ttl",
        }],
        ..CommandSpec::DEFAULT
    }
}
