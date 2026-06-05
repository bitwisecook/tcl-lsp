//! `XLAT::src_nat_valid_range` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "XLAT::src_nat_valid_range",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Return a list of valid source-translation endpoint ranges.",
            synopsis: &["XLAT::src_nat_valid_range"],
            snippet: "Returns a list of lists containing valid source-translation addresses and port-ranges (source-translation endpoints). This command must be called every time a new connection/listener needs to be created to retrieve the valid source translation information. This data must not be cached. Only the source-translation address used by the parent with a valid port-range is returned, not all of the endpoints in the source-translation object/pool. In PBA mode multiple source-translation addresses and port-ranges can be returned if the client has multiple active blocks.",
            source: "https://clouddocs.f5.com/api/irules/XLAT__src_nat_valid_range.html",
            examples: "when SA_PICKED {\n    set sa_list [XLAT::src_nat_valid_range]\n\n    foreach sa $sa_list {\n        log local0. \"address=[lindex $sa 0] port-start=[lindex $sa 1] port-end=[lindex $sa 2]\"\n    }\n}",
            return_value: "A list of lists containing the valid source-translation endpoint ranges. For e.g. { {address-1 start-port end-port} {address-2 start-port end-port} ...}",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &[],
            also_in: &[
                "CLIENT_DATA",
                "SA_PICKED",
                "SERVER_CONNECTED",
                "SERVER_DATA",
            ],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "XLAT::src_nat_valid_range" },
        ],
        ..CommandSpec::DEFAULT
    }
}
