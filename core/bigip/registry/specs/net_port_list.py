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
            "net_port_list",
            module="net",
            object_types=("port-list",),
        ),
        header_types=(("net", "port-list"),),
        properties=(
            BigipPropertySpec(
                name="ports",
                value_type="enum",
                enum_values=("add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="net", value_type="string"),
            BigipPropertySpec(name="ports", value_type="string", in_sections=("net",)),
            BigipPropertySpec(name="description", value_type="string", in_sections=("ports",)),
            BigipPropertySpec(name="http", value_type="list", in_sections=("net",), repeated=True),
            BigipPropertySpec(name="https", value_type="list", in_sections=("net",), repeated=True),
        ),
    )
