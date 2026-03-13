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
            "auth_password_policy",
            module="auth",
            object_types=("password-policy",),
        ),
        header_types=(("auth", "password-policy"),),
        properties=(
            BigipPropertySpec(name="expiration-warning", value_type="integer"),
            BigipPropertySpec(name="max-duration", value_type="integer"),
            BigipPropertySpec(name="max-login-failures", value_type="integer"),
            BigipPropertySpec(name="min-duration", value_type="integer"),
            BigipPropertySpec(name="minimum-length", value_type="integer"),
            BigipPropertySpec(name="password-memory", value_type="integer"),
            BigipPropertySpec(
                name="policy-enforcement", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="required-lowercase", value_type="integer"),
            BigipPropertySpec(name="required-numeric", value_type="integer"),
            BigipPropertySpec(name="required-special", value_type="integer"),
            BigipPropertySpec(name="required-uppercase", value_type="integer"),
            BigipPropertySpec(name="lockout-duration", value_type="integer"),
        ),
    )
