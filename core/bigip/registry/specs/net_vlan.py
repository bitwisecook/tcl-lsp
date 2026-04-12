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
            "net_vlan",
            module="net",
            object_types=("vlan",),
        ),
        header_types=(("net", "vlan"),),
        properties=(
            BigipPropertySpec(
                name="auto-lasthop",
                value_type="enum",
                enum_values=("default", "enabled", "disabled"),
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="failsafe", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(
                name="failsafe-action",
                value_type="enum",
                enum_values=("failover", "failover-restart-tm", "reboot", "restart-all"),
            ),
            BigipPropertySpec(name="failsafe-timeout", value_type="integer"),
            BigipPropertySpec(
                name="fwd-mode",
                value_type="enum",
                allow_none=True,
                enum_values=("l3", "passive", "virtual-wire", "none"),
            ),
            BigipPropertySpec(name="tag-mode", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="interfaces", value_type="boolean", allow_none=True),
            BigipPropertySpec(
                name="learning",
                value_type="enum",
                enum_values=("disable-drop", "disable-forward", "enable-forward"),
            ),
            BigipPropertySpec(name="mtu", value_type="integer"),
            BigipPropertySpec(name="nti", value_type="integer"),
            BigipPropertySpec(name="sflow", value_type="string"),
            BigipPropertySpec(name="poll-interval", value_type="integer", in_sections=("sflow",)),
            BigipPropertySpec(
                name="poll-interval-global",
                value_type="enum",
                in_sections=("sflow",),
                enum_values=("no", "yes"),
            ),
            BigipPropertySpec(name="sampling-rate", value_type="integer", in_sections=("sflow",)),
            BigipPropertySpec(
                name="sampling-rate-global",
                value_type="enum",
                in_sections=("sflow",),
                enum_values=("no", "yes"),
            ),
            BigipPropertySpec(
                name="source-checking", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="tag", value_type="integer"),
            BigipPropertySpec(name="customer-tag", value_type="boolean", allow_none=True),
            BigipPropertySpec(
                name="cmp-hash",
                value_type="enum",
                enum_values=("default", "dst-ip", "src-ip", "ipport"),
            ),
            BigipPropertySpec(name="dag-tunnel", value_type="enum", enum_values=("outer", "inner")),
            BigipPropertySpec(name="ipv6-prefix-len", value_type="integer"),
            BigipPropertySpec(
                name="dag-adjustment",
                value_type="enum",
                allow_none=True,
                enum_values=("none", "bit-roll", "xor-5mid-xor-5low", "nibble-roll"),
            ),
            BigipPropertySpec(
                name="dag-round-robin", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(
                name="hardware-syncookie", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="syncache-threshold", value_type="integer"),
            BigipPropertySpec(name="syn-flood-rate-limit", value_type="integer"),
        ),
    )
