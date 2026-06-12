//! `XLAT::src_endpoint_reservation` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "XLAT::src_endpoint_reservation",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "XLAT:src_endpoint_reservation",
            synopsis: &["XLAT::src_endpoint_reservation create", "XLAT::src_endpoint_reservation update_lifetime TRANS_ADDR TRANS_PORT LSN_POOL XLAT_PROTO XLAT_LIFETIME"],
            snippet: "Create, update, or get reserved entry values.\n\nSyntax:\nXLAT::src_endpoint_reservation create [-no-persist] [-dslite  <local> <remote>] [-pool <source translation object/pool name>] [-translation-loose|-translation-strict <ip> <port>] <client ip> <client port> <protocol> <lifetime>;\n\nCreates a reservation in the reservation table which can be viewed using the command \"lsndb list endpoint-reservation\" for the lifetime specified by the user. The command has the following characteristics:\n    1) The returned endpoint cannot be reserved for another client IP:port as long as it is active.",
            source: "https://clouddocs.f5.com/api/irules/XLAT__src_endpoint_reservation.html",
            examples: "",
            return_value: "create returns the translation endpoint used for the reservation.",
        }),
        excluded_events: &["RULE_INIT"],
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "XLAT::src_endpoint_reservation create ?options? <client_ip> <client_port> <protocol> <lifetime>" },
        ],
        options: &[
            OptionSpec { name: "-no-persist", takes_value: false, value_hint: "", detail: "Skip creation of persist entry.", dialects: None },
            OptionSpec { name: "-dslite", takes_value: true, value_hint: "LOCAL_ADDR REMOTE_ADDR", detail: "DS-Lite local and remote endpoint.", dialects: None },
            OptionSpec { name: "-pool", takes_value: true, value_hint: "POOL_NAME", detail: "Specify pool for endpoint reservation.", dialects: None },
            OptionSpec { name: "-translation-loose", takes_value: true, value_hint: "IP PORT", detail: "Hint data; command won't fail if hints can't be used.", dialects: None },
            OptionSpec { name: "-translation-strict", takes_value: true, value_hint: "IP PORT", detail: "Hint data; command fails if hints can't be used.", dialects: None },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::LsnState,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
