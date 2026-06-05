//! `DNS::scrape` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DNS::scrape",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Allows users to walk over a DNS message and parse out information from the packet based on user supplied arguments.",
            synopsis: &["DNS::scrape ('AUTHORITY' | 'ADDITIONAL' | 'ANSWER' | 'ALL') (DNS_SCRAPE_VAL)+"],
            snippet: "This iRules command allows users to walk over a DNS message and parse\nout information from the packet based on user supplied arguments.\n\nNote: This command functions only in the context of LTM iRules and\nrequires the DNS Profile, which is only enabled as part of GTM or the\nDNS Services add-on.",
            source: "https://clouddocs.f5.com/api/irules/DNS__scrape.html",
            examples: "when DNS_RESPONSE {\n   foreach rr [DNS::scrape ANSWER type ttl qnamelen rdatalen] {\n     log local2. \"ANSWER: $rr\"\n   }\n   foreach rr [DNS::scrape AUTHORITY type ttl class qnamelen rdatalen] {\n     log local2. \"AUTHORITY: $rr\"\n   }\n   foreach rr [DNS::scrape ADDITIONAL type ttl class qnamelen rdatalen] {\n     log local2. \"ADDITIONAL: $rr\"\n   }\n }",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["DNS"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        ..CommandSpec::DEFAULT
    }
}
