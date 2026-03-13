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
            "ltm_profile_dhcpv6",
            module="ltm",
            object_types=("profile dhcpv6",),
        ),
        header_types=(("ltm", "profile dhcpv6"),),
        properties=(
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                allow_none=True,
                references=("ltm_profile_dhcpv6",),
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
                enum_values=(
                    "mac-address",
                    "option37",
                    "mac-and-option37",
                    "option38",
                    "mac-and-option38",
                    "option37-and-option38",
                    "mac-and-option37-and-option38",
                    "tcl-snippet",
                ),
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
                enum_values=(
                    "mac-address",
                    "option37",
                    "mac-and-option37",
                    "option38",
                    "mac-and-option38",
                    "option37-and-option38",
                    "mac-and-option37-and-option38",
                    "tcl-snippet",
                ),
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
            BigipPropertySpec(name="remote-id-option", value_type="string"),
            BigipPropertySpec(
                name="add",
                value_type="enum",
                in_sections=("remote-id-option",),
                enum_values=("true", "false"),
            ),
            BigipPropertySpec(
                name="remove",
                value_type="enum",
                in_sections=("remote-id-option",),
                enum_values=("true", "false"),
            ),
            BigipPropertySpec(
                name="enterprise-number", value_type="integer", in_sections=("remote-id-option",)
            ),
            BigipPropertySpec(
                name="value",
                value_type="boolean",
                in_sections=("remote-id-option",),
                allow_none=True,
            ),
            BigipPropertySpec(name="subscriber-id-option", value_type="string"),
            BigipPropertySpec(
                name="add",
                value_type="enum",
                in_sections=("subscriber-id-option",),
                enum_values=("true", "false"),
            ),
            BigipPropertySpec(
                name="remove",
                value_type="enum",
                in_sections=("subscriber-id-option",),
                enum_values=("true", "false"),
            ),
            BigipPropertySpec(
                name="value",
                value_type="boolean",
                in_sections=("subscriber-id-option",),
                allow_none=True,
            ),
            BigipPropertySpec(name="reset-stats", value_type="string"),
        ),
    )
