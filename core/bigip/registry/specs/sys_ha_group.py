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
            "sys_ha_group",
            module="sys",
            object_types=("ha-group",),
        ),
        header_types=(("sys", "ha-group"),),
        properties=(
            BigipPropertySpec(name="active-bonus", value_type="integer"),
            BigipPropertySpec(name="clusters", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="attribute", value_type="string", in_sections=("clusters",)),
            BigipPropertySpec(name="threshold", value_type="integer", in_sections=("clusters",)),
            BigipPropertySpec(
                name="minimum-threshold", value_type="integer", in_sections=("clusters",)
            ),
            BigipPropertySpec(name="sufficient", value_type="integer", in_sections=("clusters",)),
            BigipPropertySpec(name="weight", value_type="integer", in_sections=("clusters",)),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="pools", value_type="reference", allow_none=True, references=("ltm_pool",)
            ),
            BigipPropertySpec(name="attribute", value_type="string", in_sections=("pools",)),
            BigipPropertySpec(name="threshold", value_type="integer", in_sections=("pools",)),
            BigipPropertySpec(
                name="minimum-threshold", value_type="integer", in_sections=("pools",)
            ),
            BigipPropertySpec(name="sufficient", value_type="integer", in_sections=("pools",)),
            BigipPropertySpec(name="weight", value_type="integer", in_sections=("pools",)),
            BigipPropertySpec(name="trunks", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="attribute", value_type="string", in_sections=("trunks",)),
            BigipPropertySpec(name="threshold", value_type="integer", in_sections=("trunks",)),
            BigipPropertySpec(
                name="minimum-threshold", value_type="integer", in_sections=("trunks",)
            ),
            BigipPropertySpec(name="sufficient", value_type="integer", in_sections=("trunks",)),
            BigipPropertySpec(name="weight", value_type="integer", in_sections=("trunks",)),
        ),
    )
