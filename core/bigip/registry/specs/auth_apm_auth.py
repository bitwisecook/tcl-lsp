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
            "auth_apm_auth",
            module="auth",
            object_types=("apm-auth",),
        ),
        header_types=(("auth", "apm-auth"),),
        properties=(BigipPropertySpec(name="profile-access", value_type="string"),),
    )
