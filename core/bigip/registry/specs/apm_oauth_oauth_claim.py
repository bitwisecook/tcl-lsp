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
            "apm_oauth_oauth_claim",
            module="apm",
            object_types=("oauth oauth-claim",),
        ),
        header_types=(("apm", "oauth oauth-claim"),),
        properties=(
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(name="claim-description", value_type="string", allow_none=True),
            BigipPropertySpec(name="claim-name", value_type="string"),
            BigipPropertySpec(
                name="claim-type",
                value_type="enum",
                enum_values=("boolean", "custom", "number"),
            ),
            BigipPropertySpec(name="claim-value", value_type="string", allow_none=True),
            BigipPropertySpec(name="options", value_type="unknown"),
        ),
    )
