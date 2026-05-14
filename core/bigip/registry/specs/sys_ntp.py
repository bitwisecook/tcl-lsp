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
            "sys_ntp",
            module="sys",
            object_types=("ntp",),
        ),
        header_types=(("sys", "ntp"),),
        properties=(
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="include", value_type="string"),
            BigipPropertySpec(
                name="restrict",
                value_type="list",
                allow_none=True,
                list_operators=frozenset(("add", "delete", "replace-all-with")),
            ),
            BigipPropertySpec(name="address", value_type="string", in_sections=("restrict",)),
            BigipPropertySpec(
                name="default-entry",
                value_type="enum",
                in_sections=("restrict",),
                enum_values=("disable", "enabled"),
            ),
            BigipPropertySpec(name="description", value_type="string", in_sections=("restrict",)),
            BigipPropertySpec(
                name="ignore",
                value_type="enum",
                in_sections=("restrict",),
                enum_values=("disable", "enabled"),
            ),
            BigipPropertySpec(
                name="kod",
                value_type="enum",
                in_sections=("restrict",),
                enum_values=("disable", "enabled"),
            ),
            BigipPropertySpec(
                name="limited",
                value_type="enum",
                in_sections=("restrict",),
                enum_values=("disable", "enabled"),
            ),
            BigipPropertySpec(
                name="low-priority-trap",
                value_type="enum",
                in_sections=("restrict",),
                enum_values=("disable", "enabled"),
            ),
            BigipPropertySpec(name="mask", value_type="string", in_sections=("restrict",)),
            BigipPropertySpec(
                name="no-modify",
                value_type="enum",
                in_sections=("restrict",),
                enum_values=("disable", "enabled"),
            ),
            BigipPropertySpec(
                name="no-peer",
                value_type="enum",
                in_sections=("restrict",),
                enum_values=("disable", "enabled"),
            ),
            BigipPropertySpec(
                name="no-query",
                value_type="enum",
                in_sections=("restrict",),
                enum_values=("disable", "enabled"),
            ),
            BigipPropertySpec(
                name="no-serve-packets",
                value_type="enum",
                in_sections=("restrict",),
                enum_values=("disable", "enabled"),
            ),
            BigipPropertySpec(
                name="no-trap",
                value_type="enum",
                in_sections=("restrict",),
                enum_values=("disable", "enabled"),
            ),
            BigipPropertySpec(
                name="no-trust",
                value_type="enum",
                in_sections=("restrict",),
                enum_values=("disable", "enabled"),
            ),
            BigipPropertySpec(
                name="non-ntp-port",
                value_type="enum",
                in_sections=("restrict",),
                enum_values=("disable", "enabled"),
            ),
            BigipPropertySpec(
                name="ntp-port",
                value_type="enum",
                in_sections=("restrict",),
                enum_values=("disable", "enabled"),
            ),
            BigipPropertySpec(
                name="version",
                value_type="enum",
                in_sections=("restrict",),
                enum_values=("disable", "enabled"),
            ),
            BigipPropertySpec(
                name="servers",
                value_type="list",
                allow_none=True,
                list_operators=frozenset(("add", "delete", "replace-all-with")),
            ),
            BigipPropertySpec(name="timezone", value_type="string"),
        ),
    )
