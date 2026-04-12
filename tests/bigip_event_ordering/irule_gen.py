"""Generate event-ordering iRules for BigIP deployment.

Produces an iRule that instruments every event in a scenario's flow
chain with a counter and timestamp, logging the firing order via
``HSL::send`` (High-Speed Logging over UDP).  Based on the approach
from e-XpertSolutions but improved with structured logging, HSL
transport, and per-scenario event filtering.

The HSL pool (``evt_order_hsl_pool``) must point to a UDP listener
running ``hsl_receiver.py``.

Priority notes:
  Within a single event, multiple ``when`` handlers may exist across
  iRules.  Execution order is: lowest priority number first (default
  500, 32-bit unsigned).  Equal priorities break by ASCII sort of the
  iRule name.  We use priority 1 so the ordering iRule fires before
  any user iRules.
"""

from __future__ import annotations

from textwrap import dedent

from .scenarios import Scenario


def generate_irule(
    scenario: Scenario,
    *,
    hsl_pool: str = "evt_order_hsl_pool",
) -> str:
    """Generate an event-ordering iRule for the given scenario.

    Returns the complete iRule source as a string.
    """
    events = scenario.all_events
    blocks: list[str] = []

    has_client_accepted = "CLIENT_ACCEPTED" in events
    for evt in events:
        extra = scenario.extra_irule_lines.get(evt, "")
        if evt == "RULE_INIT":
            blocks.append(_rule_init_block(scenario, hsl_pool))
        elif evt == "CLIENT_ACCEPTED":
            blocks.append(_client_accepted_block(extra))
        elif evt == "CLIENT_CLOSED":
            blocks.append(_client_closed_block(extra))
        elif evt == "DNS_REQUEST" and not has_client_accepted:
            # UDP DNS has no CLIENT_ACCEPTED — init session here
            blocks.append(_dns_request_block(extra))
        else:
            blocks.append(_standard_block(evt, extra))

    return "\n\n".join(blocks) + "\n"


# Block generators


def _rule_init_block(scenario: Scenario, hsl_pool: str) -> str:
    return dedent(f"""\
        when RULE_INIT priority 1 {{
            # Event ordering test: {scenario.description}
            # Priority 1 ensures this fires before all other iRules.
            # Within equal priorities, ASCII sort of iRule name determines order.
            set static::scenario "{scenario.name}"
            set static::hsl [HSL::open -proto UDP -pool {hsl_pool}]
            HSL::send $static::hsl "EVTORD scenario=$static::scenario event=RULE_INIT"
        }}""")


def _client_accepted_block(extra: str) -> str:
    extra_line = f"\n{extra}" if extra else ""
    return dedent(f"""\
        when CLIENT_ACCEPTED priority 1 {{
            # Initialise per-connection state
            set evt_counter 0
            set evt_sid "[IP::client_addr]:[TCP::client_port]:[expr {{int(100000000 * rand())}}]"
            binary scan [md5 $evt_sid] H* evt_md5
            set evt_sid [string range $evt_md5 0 8]
            set evt_start [clock clicks -milliseconds]
            HSL::send $static::hsl "EVTORD scenario=$static::scenario sid=$evt_sid seq=$evt_counter t=0 event=CLIENT_ACCEPTED"{extra_line}
        }}""")


def _dns_request_block(extra: str) -> str:
    """DNS virtual servers have no CLIENT_ACCEPTED -- init session here."""
    extra_line = f"\n{extra}" if extra else ""
    return dedent(f"""\
        when DNS_REQUEST priority 1 {{
            set evt_counter 0
            set evt_sid "[IP::client_addr]:[UDP::client_port]:[expr {{int(100000000 * rand())}}]"
            binary scan [md5 $evt_sid] H* evt_md5
            set evt_sid [string range $evt_md5 0 8]
            set evt_start [clock clicks -milliseconds]
            HSL::send $static::hsl "EVTORD scenario=$static::scenario sid=$evt_sid seq=$evt_counter t=0 event=DNS_REQUEST"{extra_line}
        }}""")


def _client_closed_block(extra: str) -> str:
    extra_line = f"\n{extra}" if extra else ""
    return dedent(f"""\
        when CLIENT_CLOSED priority 1 {{
            incr evt_counter
            set evt_dt [expr {{[clock clicks -milliseconds] - $evt_start}}]
            HSL::send $static::hsl "EVTORD scenario=$static::scenario sid=$evt_sid seq=$evt_counter t=$evt_dt event=CLIENT_CLOSED"{extra_line}
        }}""")


def _standard_block(event: str, extra: str) -> str:
    extra_line = f"\n{extra}" if extra else ""
    return dedent(f"""\
        when {event} priority 1 {{
            incr evt_counter
            set evt_dt [expr {{[clock clicks -milliseconds] - $evt_start}}]
            HSL::send $static::hsl "EVTORD scenario=$static::scenario sid=$evt_sid seq=$evt_counter t=$evt_dt event={event}"{extra_line}
        }}""")
