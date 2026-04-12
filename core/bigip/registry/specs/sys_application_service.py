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
            BigipPropertySpec(
                name="device-group",
                value_type="reference",
                allow_none=True,
                references=("cm_device_group",),
            ),
            BigipPropertySpec(name="execute-action", value_type="string"),
            BigipPropertySpec(
                name="lists",
                value_type="enum",
                enum_values=("add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="value", value_type="list", in_sections=("lists",), repeated=True
            ),
            BigipPropertySpec(
                name="encrypted",
                value_type="enum",
                in_sections=("lists",),
                enum_values=("yes", "no"),
            ),
            BigipPropertySpec(
                name="strict-updates", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(
                name="tables",
                value_type="enum",
                enum_values=("add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="column-names", value_type="list", in_sections=("tables",), repeated=True
            ),
            BigipPropertySpec(
                name="encrypted-columns", value_type="list", in_sections=("tables",), repeated=True
            ),
            BigipPropertySpec(
                name="rows", value_type="list", in_sections=("tables",), repeated=True
            ),
            BigipPropertySpec(name="rows", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="template", value_type="string"),
            BigipPropertySpec(
                name="traffic-group",
                value_type="reference",
                allow_none=True,
                references=("cm_traffic_group",),
            ),
            BigipPropertySpec(
                name="variables",
                value_type="enum",
                enum_values=("add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(name="value", value_type="string", in_sections=("variables",)),
            BigipPropertySpec(
                name="encrypted",
                value_type="enum",
                in_sections=("variables",),
                enum_values=("yes", "no"),
            ),
            BigipPropertySpec(name="value", value_type="string"),
            BigipPropertySpec(name="persist", value_type="enum", enum_values=("true", "false")),
        ),
    )
