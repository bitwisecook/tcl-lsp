//! `CACHE::trace` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "CACHE::trace",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Dump the list of cached objects for a HTTP profile where RAM Cache is enabled.",
            synopsis: &["CACHE::trace (MAX)?"],
            snippet: "Dump the list of cached objects for a HTTP profile where RAM Cache is\nenabled.\nThis event will execute only if a RAM Cache profile is enabled on the\nVirtual Server, and for objects that match the RAM Cache configuration.\nThe list will represent the size of the cache (Cache Size), number of\nobjects (Cache Count), and starting by the term Entity, it will list\nevery object:\n  * Pos (0001), list the position of the object in the cache\n  * Local Hits (00031/00007) indicate the number of Local Hits\n  * Remote Hits (00031/00007) indicate the number of Remote Hits",
            source: "https://clouddocs.f5.com/api/irules/CACHE__trace.html",
            examples: "when RULE_INIT {\n    set static::cache \"\"\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["CACHE"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "CACHE::trace (MAX)?" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::StreamProfile,
                reads: false,
                writes: true,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
