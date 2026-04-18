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
            "net_stp",
            module="net",
            object_types=("stp",),
        ),
        header_types=(("net", "stp"),),
        properties=(
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="instance-id", value_type="integer"),
            BigipPropertySpec(
                name="interfaces",
                value_type="enum",
                enum_values=("add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="external-path-cost", value_type="integer", in_sections=("interfaces",)
            ),
            BigipPropertySpec(
                name="internal-path-cost", value_type="integer", in_sections=("interfaces",)
            ),
            BigipPropertySpec(name="priority", value_type="integer", in_sections=("interfaces",)),
            BigipPropertySpec(name="priority", value_type="integer"),
            BigipPropertySpec(
                name="trunks",
                value_type="enum",
                enum_values=("add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="external-path-cost", value_type="integer", in_sections=("trunks",)
            ),
            BigipPropertySpec(
                name="internal-path-cost", value_type="integer", in_sections=("trunks",)
            ),
            BigipPropertySpec(name="priority", value_type="integer", in_sections=("trunks",)),
            BigipPropertySpec(
                name="vlans",
                value_type="reference",
                enum_values=("add", "delete", "replace-all-with"),
                references=("net_vlan",),
            ),
        ),
    )
