//! `LB::queue` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "LB::queue",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns queue information.",
            synopsis: &["LB::queue queued", "LB::queue on", "LB::queue limit", "LB::queue depth"],
            snippet: "Returns queue information. Connection queuing details:\n\n    * Operates at the TCP level\n    * Only engages when the connection limit is hit\n    * Queue is specified by length, time, or both (in the pool configuration)\n    * Queues operate per-tmm, there is no state sharing\n        * Length limit divided by tmm count\n        * FIFO guarantees only per-tmm\n    * Queued at the pool level for non-persistent connections\n    * Queued at the pool member level for persistent connections.",
            source: "https://clouddocs.f5.com/api/irules/LB__queue.html",
            examples: "when LB_QUEUED {\n    log local0. \"[IP::local_addr] was queued - [LB::queue depth one pool1] / [LB::queue limit depth pool1]\"\n}",
            return_value: "LB::queue limit depth|time [<pool name>] Returns queue limit info (depth is per-tmm)",
        }),
        ..CommandSpec::DEFAULT
    }
}
