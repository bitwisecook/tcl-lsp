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
                value_type="enum",
                enum_values=("add", "delete", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="address",
                value_type="string",
                in_sections=("restrict",),
                pattern="^(?:\\\\d{1,3}(?:\\\\.\\\\d{1,3}){3})(?:/\\\\d{1,2})?$",
            ),
            BigipPropertySpec(
                name="default-entry",
                value_type="enum",
                in_sections=("restrict",),
                enum_values=("enabled", "disable"),
            ),
            BigipPropertySpec(name="description", value_type="string", in_sections=("restrict",)),
            BigipPropertySpec(
                name="ignore",
                value_type="enum",
                in_sections=("restrict",),
                enum_values=("enabled", "disable"),
            ),
            BigipPropertySpec(
                name="kod",
                value_type="enum",
                in_sections=("restrict",),
                enum_values=("enabled", "disable"),
            ),
            BigipPropertySpec(
                name="limited",
                value_type="enum",
                in_sections=("restrict",),
                enum_values=("enabled", "disable"),
            ),
            BigipPropertySpec(
                name="low-priority-trap",
                value_type="enum",
                in_sections=("restrict",),
                enum_values=("enabled", "disable"),
            ),
            BigipPropertySpec(name="mask", value_type="string", in_sections=("restrict",)),
            BigipPropertySpec(
                name="no-modify",
                value_type="enum",
                in_sections=("restrict",),
                enum_values=("enabled", "disable"),
            ),
            BigipPropertySpec(
                name="non-ntp-port",
                value_type="enum",
                in_sections=("restrict",),
                enum_values=("enabled", "disable"),
                min_value=0,
                max_value=65535,
            ),
            BigipPropertySpec(
                name="no-peer",
                value_type="enum",
                in_sections=("restrict",),
                enum_values=("enabled", "disable"),
            ),
            BigipPropertySpec(
                name="no-query",
                value_type="enum",
                in_sections=("restrict",),
                enum_values=("enabled", "disable"),
            ),
            BigipPropertySpec(
                name="no-serve-packets",
                value_type="enum",
                in_sections=("restrict",),
                enum_values=("enabled", "disable"),
            ),
            BigipPropertySpec(
                name="no-trap",
                value_type="enum",
                in_sections=("restrict",),
                enum_values=("enabled", "disable"),
            ),
            BigipPropertySpec(
                name="no-trust",
                value_type="enum",
                in_sections=("restrict",),
                enum_values=("enabled", "disable"),
            ),
            BigipPropertySpec(
                name="ntp-port",
                value_type="enum",
                in_sections=("restrict",),
                enum_values=("enabled", "disable"),
                min_value=0,
                max_value=65535,
            ),
            BigipPropertySpec(
                name="version",
                value_type="enum",
                in_sections=("restrict",),
                enum_values=("enabled", "disable"),
            ),
            BigipPropertySpec(
                name="servers", value_type="enum", enum_values=("add", "delete", "replace-all-with")
            ),
            BigipPropertySpec(name="timezone", value_type="string"),
        ),
    )
