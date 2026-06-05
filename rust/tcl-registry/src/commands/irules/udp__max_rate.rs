//! `UDP::max_rate` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "UDP::max_rate",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "This command can be used to set/get the maximum transmission rate (bytes per second) of a UDP connection.",
            synopsis: &["UDP::max_rate (UDP_MAX_RATE)?"],
            snippet: "UDP::max_rate returns the maximum transmission rate (bytes per second) of a UDP connection.\nUDP::max_rate UDP_MAX_RATE sets the maximum transmission rate (bytes per second) to specified value.\nUDP::max_rate 0 turns off the maximum transmission rate (bytes per second) of a previously specified value.",
            source: "https://clouddocs.f5.com/api/irules/UDP__max_rate.html",
            examples: "# Get/set the max rate of the UDP flow.\nwhen CLIENT_ACCEPTED {\n    # Set the rate to 1Mbps (125,000 bytes per second)\n    log local0. \"UDP set max rate: [UDP::max_rate 125000]\"\n    log local0. \"UDP get max rate: [UDP::max_rate]\"\n}",
            return_value: "UDP::max_rate returns the maximum transmission rate (bytes per second) of a UDP connection.",
        }),
        ..CommandSpec::DEFAULT
    }
}
