//! `DNSMSG::record` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DNSMSG::record",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the specified field from a resource record object.",
            synopsis: &["DNSMSG::record RESOURCE_RECORD ('owner' | 'type' | 'ttl' | 'class' | 'rdata')"],
            snippet: "This iRule gets the specified field from a resource record object.",
            source: "https://clouddocs.f5.com/api/irules/DNSMSG-record.html",
            examples: "when CLIENT_ACCEPTED {\n        set result [RESOLVER::name_lookup \"/Common/r1\" www.abc.com a]\n        set answer [DNSMSG::section $result answer]\n        set first_rr [lindex $answer 1]\n        set rdata [DNSMSG::record $first_rr rdata]\n}",
            return_value: "Returns the specified field from the resource record object.",
        }),
        ..CommandSpec::DEFAULT
    }
}
