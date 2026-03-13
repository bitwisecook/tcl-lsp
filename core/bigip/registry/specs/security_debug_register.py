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
            "security_debug_register",
            module="security",
            object_types=("debug register",),
        ),
        header_types=(("security", "debug register"),),
        properties=(
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="destination", value_type="string"),
            BigipPropertySpec(
                name="address",
                value_type="string",
                in_sections=("destination",),
                pattern="^(?:\\\\d{1,3}(?:\\\\.\\\\d{1,3}){3})(?:/\\\\d{1,2})?$",
            ),
            BigipPropertySpec(
                name="port",
                value_type="integer",
                in_sections=("destination",),
                min_value=0,
                max_value=65535,
            ),
            BigipPropertySpec(
                name="match-ip-version", value_type="enum", enum_values=("false", "true")
            ),
            BigipPropertySpec(name="protocol", value_type="enum", enum_values=("any", "protocol")),
            BigipPropertySpec(name="source", value_type="string"),
            BigipPropertySpec(
                name="address",
                value_type="string",
                in_sections=("source",),
                pattern="^(?:\\\\d{1,3}(?:\\\\.\\\\d{1,3}){3})(?:/\\\\d{1,2})?$",
            ),
            BigipPropertySpec(
                name="port",
                value_type="integer",
                in_sections=("source",),
                min_value=0,
                max_value=65535,
            ),
            BigipPropertySpec(
                name="vlan",
                value_type="reference",
                in_sections=("source",),
                references=("net_vlan",),
            ),
            BigipPropertySpec(name="reset-stats", value_type="list", repeated=True),
            BigipPropertySpec(name="yotta", value_type="string"),
            BigipPropertySpec(name="filename", value_type="string"),
            BigipPropertySpec(name="max-file-mb", value_type="integer"),
            BigipPropertySpec(name="max-packets", value_type="integer"),
            BigipPropertySpec(
                name="unidirectional", value_type="enum", enum_values=("true", "false")
            ),
        ),
    )
