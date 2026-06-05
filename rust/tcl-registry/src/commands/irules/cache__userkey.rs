//! `CACHE::userkey` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "CACHE::userkey",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Allows users to add user-defined values to the key used by the cache to reference the cached content.",
            synopsis: &["CACHE::userkey KEY"],
            snippet: "By default, cached content is stored with a unique key referring to both\nthe URI of the resource to be cached and the User-Agent for which it\nwas formatted. If multiple variations of the same content must be\ncached under specific conditions (different client), you can use this\ncommand to create a unique key, thus creating cached content specific\nto that condition. This can be used to prevent one user or group's\ncached data from being served to different users/groups.",
            source: "https://clouddocs.f5.com/api/irules/CACHE__userkey.html",
            examples: "when HTTP_REQUEST {\n  if {[matchclass [IP::client_addr] equals $::InternalIPs]} {\n    CACHE::userkey \"Internal\"\n  } else {\n    CACHE::userkey \"External\"\n  }\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["HTTP"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "CACHE::userkey KEY" },
        ],
        ..CommandSpec::DEFAULT
    }
}
