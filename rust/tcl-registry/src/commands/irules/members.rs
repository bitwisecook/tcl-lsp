//! `members` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "members",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Lists all members of a given pool for v10.x.x.",
            synopsis: &["members ('-list')? (POOL_OBJ)"],
            snippet: "This command behaves much like active_members, but counts or lists all\nmembers (IP+port combinations) in a pool, not just active ones.\n\nNote\n\n   When assigning a snatpool to static variable and using \"members -list\"\n   to reference it in RULE_INIT, failures will be observed at startup but\n   won't show up in a reload afterwards. Expected behavior is to fail it\n   in any case as \"members -list\" is not designed to reference a snatpool\n   name.",
            source: "https://clouddocs.f5.com/api/irules/members.html",
            examples: "when HTTP_REQUEST {\n    set response \"<?xml version=\\\"1.0\\\" encoding=\\\"utf-8\\\"?><rss version=\\\"2.0\\\"><channel>\"\n    append response \"<title>BigIP Server Pool Status</title>\"\n    append response \"<description>Server Pool Status</description>\"\n    append response \"<language>en</language>\"\n    append response \"<pubDate>[clock format [clock seconds]]</pubDate>\"\n    append response \"<ttl>60</ttl>\"\n    if { [HTTP::uri] eq \"/status\" } {\n                foreach { selectedpool } [class get pooltest] {",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}
