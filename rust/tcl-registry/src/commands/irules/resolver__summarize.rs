//! `RESOLVER::summarize` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "RESOLVER::summarize",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns a summary of the response.",
            synopsis: &["RESOLVER::summarize DNS_MESSAGE"],
            snippet: "Takes a dns_message structure and returns a summary as a list of resource records.",
            source: "https://clouddocs.f5.com/api/irules/RESOLVER-summarize.html",
            examples: "when CLIENT_ACCEPTED {\n        set result [RESOLVER::name_lookup \"/Common/r1\" www.abc.com a]\n        set rrs [RESOLVER::summarize $result]\n}",
            return_value: "The summary will be a TCL list of resource record objects of the type specified in the query. Individual resource record objects are usable by the DNSMSG::record iRule command.",
        }),
        ..CommandSpec::DEFAULT
    }
}
