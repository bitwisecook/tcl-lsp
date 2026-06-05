//! `DNS::answer` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DNS::answer",
        traits: Traits::DIAGRAM_ACTION,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns, inserts, removes, or clears all RRs from the answer section.",
            synopsis: &["DNS::answer ('clear' | (('insert' | 'remove') RR_OBJECT))?"],
            snippet: "This iRules command returns, inserts, removes, or clears RRs from the\nanswer section.\n\nNote: This command functions only in the context of LTM iRules and\nrequires the DNS Profile, which is only enabled as part of GTM or the\nDNS Services add-on.",
            source: "https://clouddocs.f5.com/api/irules/DNS__answer.html",
            examples: "ttl of all answer records and add a glue record\n            when DNS_RESPONSE {\n                set rrs [DNS::answer]\n                foreach rr $rrs {\n                    DNS::ttl $rr 1234\n                }\n                set new_rr [DNS::rr \"bigip3900-30.f5net.com. 88 IN A 1.2.3.4\"]\n                DNS::additional insert $new_rr\n            }",
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
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "DNS::answer ?clear | insert <rr> | remove <rr>?" },
        ],
        ..CommandSpec::DEFAULT
    }
}
