from __future__ import annotations

from ..models import (
    BigipObjectKindSpec,
    BigipObjectSpec,
    BigipPropertySpec,
)
from ._base import register


@register
def register_spec() -> BigipObjectSpec:
    return BigipObjectSpec(
        kind_spec=BigipObjectKindSpec(
            "net_routing_debug",
            module="net",
            object_types=("routing debug",),
        ),
        header_types=(("net", "routing debug"),),
        properties=(
            BigipPropertySpec(
                name="bfd",
                value_type="enum",
                enum_values=("all", "event", "ipc-error", "ipc-event", "nsm", "packet", "session"),
            ),
            BigipPropertySpec(
                name="bgp",
                value_type="enum",
                enum_values=(
                    "all",
                    "bfd",
                    "dampening",
                    "events",
                    "filters",
                    "fsm",
                    "keepalives",
                    "nht",
                    "nsm",
                    "updates",
                    "updates-in",
                    "updates-out",
                ),
            ),
            BigipPropertySpec(
                name="nsm",
                value_type="enum",
                enum_values=(
                    "all",
                    "bfd",
                    "events",
                    "ha",
                    "ha-all",
                    "kernel",
                    "packet",
                    "packet-detail",
                    "packet-recv",
                    "packet-send",
                ),
            ),
            BigipPropertySpec(
                name="route-domain",
                value_type="reference",
                allow_none=True,
                references=("net_route_domain",),
            ),
        ),
    )
