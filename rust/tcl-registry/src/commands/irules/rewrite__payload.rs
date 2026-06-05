//! `REWRITE::payload` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "REWRITE::payload",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Queries for or manipulates REWRITE payload.",
            synopsis: &["REWRITE::payload (LENGTH | (OFFSET LENGTH))?", "REWRITE::payload length", "REWRITE::payload replace OFFSET LENGTH PAYLOAD"],
            snippet: "Queries for or manipulates REWRITE payload (content) information. With\nthis command, you can retrieve content, query for content size, or\nreplace a certain amount of content.",
            source: "https://clouddocs.f5.com/api/irules/REWRITE__payload.html",
            examples: "when REWRITE_RESPONSE_DONE {\n    # The rewrite_response_done event isn't absolutely necessary because browser will just ignore any html tags that it doesn't recongnize.\n    # However, it will be cleaner if we remove it nevertheless\n\n    set data [REWRITE::payload]\n    # Find the tags we inserted\n    set start [string first {<apm_do_not_touch>} $data]\n    set end [string last {</apm_do_not_touch>} $data]\n    # Determines the amount of characters to remove",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["REWRITE"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "REWRITE::payload (LENGTH | (OFFSET LENGTH))?" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::StreamProfile,
                reads: true,
                writes: true,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
