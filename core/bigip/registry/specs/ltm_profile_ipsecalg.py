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
            "ltm_profile_ipsecalg",
            module="ltm",
            object_types=("profile ipsecalg",),
        ),
        header_types=(("ltm", "profile ipsecalg"),),
        properties=(
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                allow_none=True,
                references=("ltm_profile_ipsecalg",),
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="idle-timeout", value_type="integer"),
            BigipPropertySpec(name="pending-ike-connection-limit", value_type="integer"),
            BigipPropertySpec(name="initial-connection-timeout", value_type="integer"),
            BigipPropertySpec(name="log-publisher", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="log-profile", value_type="boolean", allow_none=True),
        ),
    )
