//! `CACHE::useragent` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "CACHE::useragent",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Overrides the useragent value used by the cache to reference the cached content.",
            synopsis: &["CACHE::useragent AGENT"],
            snippet: "By default, cached content is stored with a unique key that\nconsists of the Host header, URI, Accept-Encoding and User-Agent.\nIf the content is generated the same for multiple User-Agents,\nthis command can be used to group various User-Agent values into a single\ngroup, thus minimizing duplicated cached content.\n\nCACHE::useragent <string>\n\n     * Overrides the useragent value used by the cache to store the cached\n       content.",
            source: "https://clouddocs.f5.com/api/irules/CACHE__useragent.html",
            examples: "when HTTP_REQUEST {\n  CACHE::useragent \"GENERIC-UA\"\n}",
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
            FormSpec { kind: FormKind::Default, synopsis: "CACHE::useragent AGENT" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::StreamProfile,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
