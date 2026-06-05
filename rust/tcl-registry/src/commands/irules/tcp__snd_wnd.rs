//! `TCP::snd_wnd` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::snd_wnd",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "The remote host's advertised receive window.",
            synopsis: &["TCP::snd_wnd"],
            snippet: "Returns the remote host's advertised receive window. If smaller\nthan the congestion window (cwnd) and send buffer size, this limits\nthe amount of outstanding data on the connection.",
            source: "https://clouddocs.f5.com/api/irules/TCP__snd_wnd.html",
            examples: "when CLIENT_CLOSED {\n    # Get Client's last advertised window.\n    log local0. \"Client's advertised rwnd: [TCP::snd_wnd]\"\n}",
            return_value: "The advertised receive window (rwnd) in bytes.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "TCP::snd_wnd" },
        ],
        ..CommandSpec::DEFAULT
    }
}
