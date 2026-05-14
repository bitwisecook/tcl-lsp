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
            "sys_application_service",
            module="sys",
            object_types=("application service",),
        ),
        header_types=(("sys", "application service"),),
        properties=(
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="device-group", value_type="string", allow_none=True),
            BigipPropertySpec(name="execute-action", value_type="reference"),
            BigipPropertySpec(
                name="lists",
                value_type="list",
                allow_none=True,
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
            ),
            BigipPropertySpec(
                name="encrypted",
                value_type="enum",
                in_sections=("lists",),
                enum_values=("no", "yes"),
            ),
            BigipPropertySpec(
                name="value",
                value_type="unknown",
                in_sections=("lists",),
                allow_none=True,
            ),
            BigipPropertySpec(name="metadata", value_type="unknown"),
            BigipPropertySpec(name="persist", value_type="enum", enum_values=("false", "true")),
            BigipPropertySpec(
                name="strict-updates",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="tables",
                value_type="list",
                allow_none=True,
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
            ),
            BigipPropertySpec(name="column-names", value_type="list", in_sections=("tables",)),
            BigipPropertySpec(name="encrypted-columns", value_type="list", in_sections=("tables",)),
            BigipPropertySpec(
                name="rows",
                value_type="list",
                in_sections=("tables",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="template",
                value_type="reference",
                references=(
                    "security_bot_defense_template",
                    "sys_application_template",
                    "vcmp_virtual_disk_template",
                ),
            ),
            BigipPropertySpec(
                name="traffic-group",
                value_type="string",
                allow_none=True,
                references=("cm_traffic_group",),
            ),
            BigipPropertySpec(name="value", value_type="string"),
            BigipPropertySpec(
                name="variables",
                value_type="list",
                allow_none=True,
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
            ),
            BigipPropertySpec(
                name="encrypted",
                value_type="enum",
                in_sections=("variables",),
                enum_values=("no", "yes"),
            ),
            BigipPropertySpec(name="value", value_type="string", in_sections=("variables",)),
        ),
    )
