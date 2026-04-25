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
            "net_interface",
            module="net",
            object_types=("interface",),
        ),
        header_types=(("net", "interface"),),
        properties=(
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="flow-control", value_type="boolean", allow_none=True),
            BigipPropertySpec(
                name="force-gigabit-fiber", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(
                name="forward-error-correction",
                value_type="enum",
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(name="media", value_type="string"),
            BigipPropertySpec(name="media-fixed", value_type="string"),
            BigipPropertySpec(name="media-sfp", value_type="string"),
            BigipPropertySpec(name="none", value_type="boolean"),
            BigipPropertySpec(
                name="port-fwd-mode",
                value_type="enum",
                enum_values=("l3", "passive", "virtual-wire"),
            ),
            BigipPropertySpec(
                name="prefer-port",
                value_type="enum",
                enum_values=("fixed", "sfp"),
                min_value=0,
                max_value=65535,
            ),
            BigipPropertySpec(name="sflow", value_type="string"),
            BigipPropertySpec(name="poll-interval", value_type="integer", in_sections=("sflow",)),
            BigipPropertySpec(
                name="poll-interval-global",
                value_type="enum",
                in_sections=("sflow",),
                enum_values=("no", "yes"),
            ),
            BigipPropertySpec(name="lacp-port-priority", value_type="integer"),
            BigipPropertySpec(
                name="link-traps-enabled", value_type="enum", enum_values=("false", "true")
            ),
            BigipPropertySpec(
                name="lldp-admin",
                value_type="enum",
                enum_values=("disable", "rxonly", "txonly", "txrx"),
            ),
            BigipPropertySpec(name="lldp-tlvmap", value_type="integer"),
            BigipPropertySpec(name="stp", value_type="enum", enum_values=("disabled", "enabled")),
            BigipPropertySpec(
                name="stp-auto-edge-port",
                value_type="enum",
                enum_values=("enabled", "disabled"),
                min_value=0,
                max_value=65535,
            ),
            BigipPropertySpec(
                name="stp-edge-port",
                value_type="enum",
                enum_values=("false", "true"),
                min_value=0,
                max_value=65535,
            ),
            BigipPropertySpec(
                name="stp-link-type", value_type="enum", enum_values=("auto", "p2p", "shared")
            ),
            BigipPropertySpec(name="span-mode", value_type="enum", enum_values=("false", "true")),
            BigipPropertySpec(name="qinq-ethertype", value_type="string"),
            BigipPropertySpec(
                name="bundle",
                value_type="enum",
                enum_values=("disabled", "enabled", "not-supported"),
            ),
            BigipPropertySpec(
                name="bundle-speed", value_type="enum", enum_values=("100g", "40g", "not-supported")
            ),
            BigipPropertySpec(name="reset-stats", value_type="string"),
        ),
    )
