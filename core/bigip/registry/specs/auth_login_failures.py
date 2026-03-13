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
            "auth_login_failures",
            module="auth",
            object_types=("login-failures",),
        ),
        header_types=(("auth", "login-failures"),),
        properties=(BigipPropertySpec(name="reset-stats", value_type="string"),),
    )
