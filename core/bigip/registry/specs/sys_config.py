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
            "sys_config",
            module="sys",
            object_types=("config",),
        ),
        header_types=(("sys", "config"),),
        properties=(
            BigipPropertySpec(name="save", value_type="string"),
            BigipPropertySpec(name="load", value_type="string"),
            BigipPropertySpec(name="net", value_type="string"),
            BigipPropertySpec(name="defaults", value_type="string", in_sections=("net",)),
            BigipPropertySpec(name="config-name", value_type="string", in_sections=("net",)),
            BigipPropertySpec(name="sys", value_type="list", repeated=True),
            BigipPropertySpec(
                name="gateway",
                value_type="reference",
                in_sections=("sys",),
                pattern="^(?:\\\\d{1,3}(?:\\\\.\\\\d{1,3}){3})(?:/\\\\d{1,2})?$",
                references=("net_self", "net_route", "ltm_virtual_address"),
            ),
            BigipPropertySpec(name="level", value_type="boolean", in_sections=("sys",)),
            BigipPropertySpec(name="ltm", value_type="string"),
            BigipPropertySpec(name="slow-ramp-time", value_type="string", in_sections=("ltm",)),
        ),
    )
