//! `TCP::snd_ssthresh` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::snd_ssthresh",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the TCP slow start threshold (ssthresh).",
            synopsis: &["TCP::snd_ssthresh"],
            snippet: "The slow start threshold (ssthresh) is the point at which the\ncongestion window (cwnd) grows less aggressively. When the cwnd is\nless than ssthresh, it roughly doubles for every cwnd worth of\nacknowledged data. When cwnd is greater than ssthresh, it increases\nby approximately one MSS for each cwnd worth of acknowledged data.\n\nssthresh starts at 1,073,725,440 bytes unless there is a cmetrics\ncache entry. When TCP detects packet loss it usually sets ssthresh\nto a value between 1/2 cwnd and cwnd, depending on  connection\nconditions and the congestion control algorithm.",
            source: "https://clouddocs.f5.com/api/irules/TCP__snd_ssthresh.html",
            examples: "when CLIENT_CLOSED {\n    # Get BIGIP's last slow-start threshold.\n    log local0. \"BIGIP's ssthresh: [TCP::snd_ssthresh]\"\n}",
            return_value: "The connection slow start threshold in bytes.",
        }),
        ..CommandSpec::DEFAULT
    }
}
