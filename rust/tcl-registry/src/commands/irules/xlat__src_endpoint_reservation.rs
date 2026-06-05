//! `XLAT::src_endpoint_reservation` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
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
        ..CommandSpec::DEFAULT
    }
}
