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
            BigipPropertySpec(name="app-service", value_type="string", allow_none=True),
            BigipPropertySpec(name="authentication", value_type="unknown"),
            BigipPropertySpec(
                name="enabled",
                value_type="enum",
                in_sections=("authentication",),
                enum_values=("false", "true"),
            ),
            BigipPropertySpec(
                name="user-name",
                value_type="unknown",
                in_sections=("authentication",),
            ),
            BigipPropertySpec(
                name="format",
                value_type="enum",
                in_sections=("authentication", "user-name"),
                enum_values=(
                    "address",
                    "as",
                    "concatenation",
                    "defined",
                    "is",
                    "mac-and-relay-option",
                    "option",
                    "relay-agent",
                    "relay-option",
                    "separator1",
                    "symbol",
                    "tcl-snippet",
                ),
            ),
            BigipPropertySpec(
                name="separator1",
                value_type="string",
                in_sections=("authentication", "user-name"),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="separator2",
                value_type="string",
                in_sections=("authentication", "user-name"),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="suboption-id1",
                value_type="integer",
                in_sections=("authentication", "user-name"),
            ),
            BigipPropertySpec(
                name="suboption-id2",
                value_type="integer",
                in_sections=("authentication", "user-name"),
            ),
            BigipPropertySpec(
                name="tcl",
                value_type="string",
                in_sections=("authentication", "user-name"),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="virtual",
                value_type="string",
                in_sections=("authentication",),
                allow_none=True,
            ),
            BigipPropertySpec(name="default-lease-time", value_type="integer"),
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                allow_none=True,
                references=("ltm_profile_dhcpv4",),
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="idle-timeout", value_type="integer"),
            BigipPropertySpec(name="lease-query-max-retry", value_type="integer"),
            BigipPropertySpec(
                name="lease-query-only",
                value_type="enum",
                enum_values=("false", "true"),
            ),
            BigipPropertySpec(name="max-hops", value_type="integer"),
            BigipPropertySpec(name="mode", value_type="enum", enum_values=("forwarding", "relay")),
            BigipPropertySpec(name="relay-agent-id", value_type="unknown"),
            BigipPropertySpec(
                name="add",
                value_type="enum",
                in_sections=("relay-agent-id",),
                enum_values=("false", "true"),
            ),
            BigipPropertySpec(
                name="remove",
                value_type="enum",
                in_sections=("relay-agent-id",),
                enum_values=("false", "true"),
            ),
            BigipPropertySpec(
                name="suboption",
                value_type="unknown",
                in_sections=("relay-agent-id",),
            ),
            BigipPropertySpec(
                name="id1",
                value_type="integer",
                in_sections=("relay-agent-id", "suboption"),
            ),
            BigipPropertySpec(
                name="id2",
                value_type="integer",
                in_sections=("relay-agent-id", "suboption"),
            ),
            BigipPropertySpec(
                name="value1",
                value_type="enum",
                in_sections=("relay-agent-id", "suboption"),
                allow_none=True,
                enum_values=("none",),
            ),
            BigipPropertySpec(
                name="value2",
                value_type="enum",
                in_sections=("relay-agent-id", "suboption"),
                allow_none=True,
                enum_values=("none",),
            ),
            BigipPropertySpec(name="subscriber-discovery", value_type="unknown"),
            BigipPropertySpec(
                name="enabled",
                value_type="enum",
                in_sections=("subscriber-discovery",),
                enum_values=("false", "true"),
            ),
            BigipPropertySpec(
                name="subscriber-id",
                value_type="unknown",
                in_sections=("subscriber-discovery",),
            ),
            BigipPropertySpec(
                name="format",
                value_type="enum",
                in_sections=("subscriber-discovery", "subscriber-id"),
                enum_values=(
                    "address",
                    "as",
                    "concatenation",
                    "defined",
                    "is",
                    "mac-and-relay-id",
                    "option",
                    "relay-agent",
                    "separator1",
                    "symbol",
                    "tcl-snippet",
                ),
            ),
            BigipPropertySpec(
                name="separator1",
                value_type="string",
                in_sections=("subscriber-discovery", "subscriber-id"),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="separator2",
                value_type="string",
                in_sections=("subscriber-discovery", "subscriber-id"),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="suboption-id1",
                value_type="integer",
                in_sections=("subscriber-discovery", "subscriber-id"),
            ),
            BigipPropertySpec(
                name="suboption-id2",
                value_type="integer",
                in_sections=("subscriber-discovery", "subscriber-id"),
            ),
            BigipPropertySpec(
                name="tcl",
                value_type="string",
                in_sections=("subscriber-discovery", "subscriber-id"),
                allow_none=True,
            ),
            BigipPropertySpec(name="transaction-timeout", value_type="integer"),
            BigipPropertySpec(
                name="ttl-dec-value",
                value_type="enum",
                enum_values=("by-0", "by-1", "by-2", "by-4"),
            ),
            BigipPropertySpec(name="ttl-value", value_type="integer"),
        ),
    )
