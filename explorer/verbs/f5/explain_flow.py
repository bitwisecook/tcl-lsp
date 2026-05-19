"""``f5 explain-flow`` — trace each flow in a PCAP through the BIG-IP config.

For every unique 5-tuple in the capture, find the virtual server whose
destination matches and emit the per-flow plan: profiles in attach
order, attached LTM policies and APM access profile, the expected
iRule event firing sequence (driven by attached profiles + L7 features
seen in the capture), the verbatim ``when EVENT { … }`` body for each
event, persistence, SNAT, default pool & members, and any GTM wide-IPs
present in the config.

If ``tshark`` is on PATH the verb can call it (``--tshark``) to enrich
flows with HTTP method / Host / URI and TLS SNI; absence of tshark
degrades gracefully via the built-in libpcap+pcapng walker that
already handles common TLS ClientHello / HTTP request prefixes.
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

from dialects.f5.bigip.explain_flow import (
    compute_explain_flow,
    report_to_dict,
    tshark_available,
)

from ._paths import load_paths
from ._registry import verb


@verb(
    "explain-flow",
    help="Trace each flow in a PCAP through the BIG-IP config (VS, profiles, iRules, pool, SNAT, GTM, APM).",
    formatter_class=argparse.RawDescriptionHelpFormatter,
)
def _configure(p: argparse.ArgumentParser, *, prog_name: str, default_dialect: str) -> None:  # noqa: ARG001
    p.description = (
        "For every unique flow in PCAP, find the matching virtual\n"
        "server (by destination IP:port), and print: ordered profiles,\n"
        "attached LTM policies, the iRule event firing order expected\n"
        "for the observed traffic (TCP / TLS / HTTP), each `when EVENT\n"
        "{ ... }` body that would fire, persistence, SNAT, default pool\n"
        "and pool members, plus any GTM wide-IPs and APM profile.\n"
        "Optionally calls tshark for richer L7 decoding."
    )
    p.epilog = (
        "Examples:\n"
        "  f5 explain-flow capture.pcap bigip.conf\n"
        "  f5 explain-flow capture.pcapng bigip.conf --tshark\n"
        "  f5 explain-flow capture.pcapng bigip.conf --tshark --keylog ssl.log\n"
        "  f5 explain-flow capture.pcap bigip.conf --simulate -o report.txt\n"
        "  f5 explain-flow capture.pcap bigip.conf --no-event-bodies --json\n"
    )
    p.add_argument("pcap", help="PCAP file to analyse (libpcap or pcapng).")
    p.add_argument("paths", nargs="+", help="bigip.conf / SCF files (`-` for stdin).")
    p.add_argument(
        "--tshark",
        action="store_true",
        help="Enrich flows via tshark (HTTP method/Host/URI/response code, "
        "TLS SNI/version/cipher/ALPN, TLS alerts, F5 reset cause).  "
        "Requires `tshark` on PATH; silently no-ops otherwise.",
    )
    p.add_argument(
        "--keylog",
        metavar="FILE",
        default="",
        help="NSS-format TLS keylog file (SSLKEYLOGFILE).  When supplied, "
        "passed to tshark as `-o tls.keylog_file:<FILE>` so HTTPS "
        "payloads decrypt and HTTP request/response decoding works on "
        "TLS-wrapped sessions.  Implies --tshark.",
    )
    p.add_argument(
        "--tshark-filter",
        metavar="FILTER",
        default="",
        help="tshark display filter applied via -Y; only matching packets "
        "contribute to the flow set.  When set, the entire extraction "
        "pass runs through tshark (the built-in walker is bypassed) "
        "so reassembly + decryption (with --keylog) cover what raw "
        "packet sniffing can't.  Note: F5 trailer peer-tuples for "
        "`:np` capture pairing are only populated by the built-in "
        "walker, so a filtered run loses front/back session pairing.",
    )
    p.add_argument(
        "--simulate",
        action="store_true",
        help="Run each matched session's iRule under the C-tcl test "
        "orchestrator with the captured HTTP/TLS state, and report the "
        "actual pool/node selection, HTTP::respond decisions, log lines "
        "and other side-effects.  Slower (spins a tclsh subprocess per "
        "matched session); requires `tclsh` on PATH.",
    )
    p.add_argument(
        "--no-event-bodies",
        action="store_true",
        help="Suppress the verbatim `when EVENT { ... }` block for each "
        "fired event; just list the event names.",
    )
    p.add_argument(
        "--max-event-lines",
        type=int,
        default=40,
        metavar="N",
        help="Truncate each event body after N lines (default 40).",
    )
    p.add_argument("--json", action="store_true", help="Emit JSON instead of text.")
    p.add_argument("-o", "--output", metavar="FILE", help="Write report here (default: stdout).")
    p.set_defaults(handler=_run_explain_flow)


def _run_explain_flow(args: argparse.Namespace) -> int:
    pcap_path = Path(args.pcap)
    if not pcap_path.is_file():
        print(f"error: not a file: {args.pcap}", file=sys.stderr)
        return 2

    try:
        _sources, configs = load_paths(args.paths)
    except OSError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 2

    use_tshark = args.tshark or bool(args.keylog) or bool(args.tshark_filter)
    if use_tshark and not tshark_available():
        print(
            "warning: --tshark/--keylog requested but `tshark` not on PATH; "
            "continuing with built-in walker only.",
            file=sys.stderr,
        )
    if args.keylog and not Path(args.keylog).is_file():
        print(f"warning: keylog file not found: {args.keylog}", file=sys.stderr)

    try:
        report = compute_explain_flow(
            pcap_path,
            configs,
            use_tshark=use_tshark,
            keylog_path=args.keylog,
            tshark_filter=args.tshark_filter,
            show_event_bodies=not args.no_event_bodies,
            max_event_body_lines=args.max_event_lines,
            simulate=args.simulate,
        )
    except (OSError, ValueError) as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 2

    if args.json:
        output = json.dumps(report_to_dict(report), indent=2) + "\n"
    else:
        output = report.text_report

    if args.output:
        Path(args.output).write_text(output, encoding="utf-8")
    else:
        sys.stdout.write(output)

    return 0 if report.matched_count > 0 else 1
