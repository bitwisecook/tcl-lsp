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
            "pem_profile_radius_aaa",
            module="pem",
            object_types=("profile radius-aaa",),
        ),
        header_types=(("pem", "profile radius-aaa"),),
        properties=(
            BigipPropertySpec(name="app-service", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                allow_none=True,
                references=("pem_profile_radius_aaa",),
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="password", value_type="string"),
            BigipPropertySpec(name="retransmission-timeout", value_type="integer"),
            BigipPropertySpec(name="shared-secret", value_type="string"),
            BigipPropertySpec(name="transaction-timeout", value_type="integer"),
        ),
    )
