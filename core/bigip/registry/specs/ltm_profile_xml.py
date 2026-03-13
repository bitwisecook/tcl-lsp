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
            "ltm_profile_xml",
            module="ltm",
            object_types=("profile xml",),
        ),
        header_types=(("ltm", "profile xml"),),
        properties=(
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                allow_none=True,
                references=("ltm_profile_xml",),
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="namespace-mappings", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="xpath-queries", value_type="boolean", allow_none=True),
            BigipPropertySpec(
                name="multiple-query-matches",
                value_type="enum",
                enum_values=("enabled", "disabled"),
            ),
        ),
    )
