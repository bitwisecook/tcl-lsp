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
            BigipPropertySpec(
                name="bundle",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(name="bundle-speed", value_type="enum", enum_values=("100G", "40G")),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="flow-control", value_type="unknown"),
            BigipPropertySpec(
                name="force-gigabit-fiber",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="forward-error-correction",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(name="lacp-port-priority", value_type="integer"),
            BigipPropertySpec(
                name="link-traps-enabled",
                value_type="enum",
                enum_values=("false", "true"),
            ),
            BigipPropertySpec(
                name="lldp-admin",
                value_type="enum",
                enum_values=("disable", "rxonly", "txonly", "txrx"),
            ),
            BigipPropertySpec(name="lldp-tlvmap", value_type="integer"),
            BigipPropertySpec(name="media", value_type="enum", enum_values=("auto", "no-phy")),
            BigipPropertySpec(
                name="media-fixed",
                value_type="enum",
                enum_values=("auto", "no-phy"),
            ),
            BigipPropertySpec(
                name="media-sfp",
                value_type="enum",
                allow_none=True,
                enum_values=("auto", "no-phy", "none"),
            ),
            BigipPropertySpec(name="no-mgmt", value_type="unknown"),
            BigipPropertySpec(
                name="port-fwd-mode",
                value_type="enum",
                enum_values=("l3", "passive", "virtual-wire"),
            ),
            BigipPropertySpec(name="prefer-port", value_type="enum", enum_values=("fixed", "sfp")),
            BigipPropertySpec(name="qinq-ethertype", value_type="string"),
            BigipPropertySpec(name="sflow", value_type="unknown"),
            BigipPropertySpec(name="poll-interval", value_type="integer", in_sections=("sflow",)),
            BigipPropertySpec(
                name="poll-interval-global",
                value_type="enum",
                in_sections=("sflow",),
                enum_values=("no", "yes"),
            ),
            BigipPropertySpec(name="span-mode", value_type="enum", enum_values=("false", "true")),
            BigipPropertySpec(name="stp", value_type="enum", enum_values=("disabled", "enabled")),
            BigipPropertySpec(
                name="stp-auto-edge-port",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="stp-edge-port",
                value_type="enum",
                enum_values=("false", "true"),
            ),
            BigipPropertySpec(
                name="stp-link-type",
                value_type="enum",
                enum_values=("auto", "p2p", "shared"),
            ),
            BigipPropertySpec(name="stp-reset", value_type="unknown"),
        ),
    )
