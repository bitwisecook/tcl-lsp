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
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="auto-lasthop",
                value_type="enum",
                enum_values=("default", "disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="cmp-hash",
                value_type="enum",
                enum_values=("default", "dst-ip", "src-ip"),
            ),
            BigipPropertySpec(
                name="customer-tag",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="dag-adjustment",
                value_type="enum",
                allow_none=True,
                enum_values=("bit-roll", "nibble-roll", "none", "xor-5mid-xor-5low"),
            ),
            BigipPropertySpec(
                name="dag-round-robin",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="dag-tunnel",
                value_type="enum",
                enum_values=("inner", "outer"),
                default="outer",
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="failsafe",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(
                name="failsafe-action",
                value_type="enum",
                enum_values=("failover", "failover-restart-tm", "reboot", "restart-all"),
                default="failover-restart-tm",
            ),
            BigipPropertySpec(name="failsafe-timeout", value_type="integer", default="90 seconds"),
            BigipPropertySpec(
                name="fwd-mode",
                value_type="enum",
                allow_none=True,
                enum_values=("l3", "none", "passive", "virtual-wire"),
                usage_flags=frozenset(("read_only",)),
            ),
            BigipPropertySpec(
                name="hardware-syncookie",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(name="interfaces", value_type="unknown", allow_none=True),
            BigipPropertySpec(name="ipv6-prefix-len", value_type="integer"),
            BigipPropertySpec(
                name="learning",
                value_type="enum",
                enum_values=("disable-drop", "disable-forward", "enable-forward"),
                default="enable-forward",
            ),
            BigipPropertySpec(name="mtu", value_type="integer", default="1500"),
            BigipPropertySpec(name="nti", value_type="integer", default="4096"),
            BigipPropertySpec(
                name="sflow",
                value_type="unknown",
                shape_kind="object",
                block=(
                    BigipPropertySpec(
                        name="poll-interval",
                        value_type="integer",
                        in_sections=("sflow",),
                        default="0",
                    ),
                    BigipPropertySpec(
                        name="poll-interval-global",
                        value_type="enum",
                        in_sections=("sflow",),
                        enum_values=("no", "yes"),
                        shape_kind="boolean",
                        default="yes",
                    ),
                    BigipPropertySpec(
                        name="sampling-rate",
                        value_type="integer",
                        in_sections=("sflow",),
                        default="0",
                    ),
                    BigipPropertySpec(
                        name="sampling-rate-global",
                        value_type="enum",
                        in_sections=("sflow",),
                        enum_values=("no", "yes"),
                        shape_kind="boolean",
                        default="yes",
                    ),
                ),
            ),
            BigipPropertySpec(
                name="poll-interval",
                value_type="integer",
                in_sections=("sflow",),
                default="0",
            ),
            BigipPropertySpec(
                name="poll-interval-global",
                value_type="enum",
                in_sections=("sflow",),
                enum_values=("no", "yes"),
                shape_kind="boolean",
                default="yes",
            ),
            BigipPropertySpec(
                name="sampling-rate",
                value_type="integer",
                in_sections=("sflow",),
                default="0",
            ),
            BigipPropertySpec(
                name="sampling-rate-global",
                value_type="enum",
                in_sections=("sflow",),
                enum_values=("no", "yes"),
                shape_kind="boolean",
                default="yes",
            ),
            BigipPropertySpec(
                name="source-checking",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(
                name="syn-flood-rate-limit",
                value_type="integer",
                default="set at 1000 packets per second",
            ),
            BigipPropertySpec(
                name="syncache-threshold",
                value_type="integer",
                default="set to 6000 packets",
            ),
            BigipPropertySpec(
                name="tag",
                value_type="enum",
                enum_values=("4096",),
                default="to not use this option, and the system assigns a tag number between 1 to 4094",
            ),
            BigipPropertySpec(
                name="tag-mode",
                value_type="enum",
                allow_none=True,
                enum_values=("customer", "double", "none", "service"),
                default="none",
            ),
        ),
    )
