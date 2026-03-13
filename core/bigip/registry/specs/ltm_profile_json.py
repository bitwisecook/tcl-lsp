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
            "ltm_profile_json",
            module="ltm",
            object_types=("profile json",),
        ),
        header_types=(("ltm", "profile json"),),
        properties=(
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                allow_none=True,
                references=("ltm_profile_json",),
            ),
            BigipPropertySpec(name="description", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="maximum-bytes", value_type="integer"),
            BigipPropertySpec(name="maximum-entries", value_type="integer"),
            BigipPropertySpec(name="maximum-non-json-bytes", value_type="integer"),
        ),
    )
