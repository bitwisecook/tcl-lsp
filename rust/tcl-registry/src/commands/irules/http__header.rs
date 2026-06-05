//! `HTTP::header` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP::header",
        traits: Traits::PURE | Traits::CSE_CANDIDATE | Traits::DIAGRAM_ACTION,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(1),
        options: &[OptionSpec {
            name: "-noupdate",
            takes_value: false,
            value_hint: "",
            detail: "Do not propagate the header mutation to subsequent BIG-IP filters.",
            dialects: None,
        }],
        hover: Some(HoverSnippet {
            summary: "Inspect or mutate HTTP headers in an iRule event.",
            synopsis: &["HTTP::header <subcommand> ?arg ...?"],
            snippet: "Use subcommands like `value`, `insert`, `replace`, and `remove`.",
            source: "https://clouddocs.f5.com/api/irules/HTTP__header.html",
            examples: "",
            return_value: "",
        }),
        // GAP-D2: `HTTP::header insert|replace` with tainted data →
        // header injection (IRULE3002). Mirrors `irules/http__header.py`.
        // TODO(consumer): GAP-3a — once the iRules subcommands are
        // re-ported, attach `credential_arg=2` + `sensitive_headers` to
        // the `insert` / `replace` SubCommand specs.
        taint_output_sink: Some("IRULE3002"),
        taint_output_sink_subcommands: &["insert", "replace"],
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: Some("tcp"),
            profiles: &["FASTHTTP", "HTTP"],
            also_in: &["MR_EGRESS", "MR_INGRESS", "SERVER_CONNECTED"],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "HTTP::header <subcommand> ?arg ...?",
        }],
        ..CommandSpec::DEFAULT
    }
}
