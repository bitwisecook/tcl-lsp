//! `DNS::additional` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DNS::additional",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns, inserts, removes, or clears RRs from the additional section.",
            synopsis: &["DNS::additional ('clear' | (('insert' | 'remove') RR_OBJECT))?"],
            snippet: "This iRules command returns, inserts, removes, or clears RRs from the\nadditional section.\n\nNote: This command functions only in the context of LTM iRules and\nrequires the DNS Profile, which is only enabled as part of GTM or the\nDNS Services add-on.",
            source: "https://clouddocs.f5.com/api/irules/DNS__additional.html",
            examples: "when DNS_RESPONSE {\n        set rrs [DNS::answer]\n        foreach rr $rrs {\n            DNS::ttl $rr 1234\n        }\n        set new_rr [DNS::rr \"bigip3900-30.f5net.com. 88 IN A 1.2.3.4\"]\n        DNS::additional insert $new_rr\n    }",
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
