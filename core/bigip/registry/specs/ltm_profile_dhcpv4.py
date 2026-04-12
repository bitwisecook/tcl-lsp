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
            "ltm_profile_dhcpv4",
            module="ltm",
            object_types=("profile dhcpv4",),
        ),
        header_types=(("ltm", "profile dhcpv4"),),
        properties=(
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                allow_none=True,
                references=("ltm_profile_dhcpv4",),
            ),
            BigipPropertySpec(name="mode", value_type="enum", enum_values=("relay", "forwarding")),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="idle-timeout", value_type="integer"),
            BigipPropertySpec(name="default-lease-time", value_type="integer"),
            BigipPropertySpec(name="lease-query-max-retry", value_type="integer"),
            BigipPropertySpec(
                name="lease-query-only", value_type="enum", enum_values=("true", "false")
            ),
            BigipPropertySpec(name="transaction-timeout", value_type="integer"),
            BigipPropertySpec(name="authentication", value_type="string"),
            BigipPropertySpec(
                name="enabled",
                value_type="enum",
                in_sections=("authentication",),
                enum_values=("true", "false"),
            ),
            BigipPropertySpec(
                name="virtual",
                value_type="boolean",
                in_sections=("authentication",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="user-name", value_type="string", in_sections=("authentication",)
            ),
            BigipPropertySpec(
                name="format",
                value_type="enum",
                in_sections=("user-name",),
                enum_values=("mac-address", "mac-and-relay-option", "relay-option", "tcl-snippet"),
            ),
            BigipPropertySpec(
                name="suboption-id1", value_type="integer", in_sections=("user-name",)
            ),
            BigipPropertySpec(
                name="suboption-id2", value_type="integer", in_sections=("user-name",)
            ),
            BigipPropertySpec(
                name="separator1", value_type="boolean", in_sections=("user-name",), allow_none=True
            ),
            BigipPropertySpec(
                name="separator2", value_type="boolean", in_sections=("user-name",), allow_none=True
            ),
            BigipPropertySpec(
                name="tcl", value_type="boolean", in_sections=("user-name",), allow_none=True
            ),
            BigipPropertySpec(name="subscriber-discovery", value_type="string"),
            BigipPropertySpec(
                name="enabled",
                value_type="enum",
                in_sections=("subscriber-discovery",),
                enum_values=("true", "false"),
            ),
            BigipPropertySpec(
                name="subscriber-id", value_type="integer", in_sections=("subscriber-discovery",)
            ),
            BigipPropertySpec(
                name="format",
                value_type="enum",
                in_sections=("subscriber-id",),
                enum_values=("mac-address", "mac-and-relay-id", "tcl-snippet"),
            ),
            BigipPropertySpec(
                name="suboption-id1", value_type="integer", in_sections=("subscriber-id",)
            ),
            BigipPropertySpec(
                name="suboption-id2", value_type="integer", in_sections=("subscriber-id",)
            ),
            BigipPropertySpec(
                name="separator1",
                value_type="boolean",
                in_sections=("subscriber-id",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="separator2",
                value_type="boolean",
                in_sections=("subscriber-id",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="tcl", value_type="boolean", in_sections=("subscriber-id",), allow_none=True
            ),
            BigipPropertySpec(name="relay-agent-id", value_type="integer"),
            BigipPropertySpec(
                name="add",
                value_type="enum",
                in_sections=("relay-agent-id",),
                enum_values=("true", "false"),
            ),
            BigipPropertySpec(
                name="remove",
                value_type="enum",
                in_sections=("relay-agent-id",),
                enum_values=("true", "false"),
            ),
            BigipPropertySpec(
                name="suboption", value_type="string", in_sections=("relay-agent-id",)
            ),
            BigipPropertySpec(name="id1", value_type="integer", in_sections=("suboption",)),
            BigipPropertySpec(name="id2", value_type="integer", in_sections=("suboption",)),
            BigipPropertySpec(
                name="value1", value_type="boolean", in_sections=("suboption",), allow_none=True
            ),
            BigipPropertySpec(
                name="value2", value_type="boolean", in_sections=("suboption",), allow_none=True
            ),
            BigipPropertySpec(name="ttl-value", value_type="integer"),
            BigipPropertySpec(
                name="ttl-dec-value",
                value_type="enum",
                enum_values=("by-0", "by-1", "by-2", "by-4"),
            ),
            BigipPropertySpec(name="max-hops", value_type="integer"),
            BigipPropertySpec(name="reset-stats", value_type="string"),
        ),
    )
