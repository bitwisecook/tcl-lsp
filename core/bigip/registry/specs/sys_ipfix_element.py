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
            "sys_ipfix_element",
            module="sys",
            object_types=("ipfix element",),
        ),
        header_types=(("sys", "ipfix element"),),
        properties=(
            BigipPropertySpec(name="datetime-milliseconds", value_type="boolean"),
            BigipPropertySpec(name="datetime-seconds", value_type="string"),
            BigipPropertySpec(
                name="ipv4-address",
                value_type="string",
                pattern="^(?:\\\\d{1,3}(?:\\\\.\\\\d{1,3}){3})(?:/\\\\d{1,2})?$",
            ),
            BigipPropertySpec(name="octetarray", value_type="string"),
            BigipPropertySpec(name="signed8", value_type="string"),
            BigipPropertySpec(name="unsigned16", value_type="string"),
            BigipPropertySpec(name="unsigned64", value_type="string"),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="enterprise-id", value_type="integer"),
            BigipPropertySpec(name="id", value_type="integer"),
            BigipPropertySpec(name="size", value_type="integer"),
        ),
    )
