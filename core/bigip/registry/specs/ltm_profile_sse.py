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
            "ltm_profile_sse",
            module="ltm",
            object_types=("profile sse",),
        ),
        header_types=(("ltm", "profile sse"),),
        properties=(
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                allow_none=True,
                references=("ltm_profile_sse",),
            ),
            BigipPropertySpec(name="description", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="max-field-name-size", value_type="integer"),
            BigipPropertySpec(name="max-buffered-msg-bytes", value_type="integer"),
        ),
    )
