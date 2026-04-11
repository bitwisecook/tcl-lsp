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
            "ltm_auth_radius",
            module="ltm",
            object_types=("auth radius",),
        ),
        header_types=(("ltm", "auth radius"),),
        properties=(
            BigipPropertySpec(
                name="accounting-bug", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="client-id", value_type="integer", allow_none=True),
            BigipPropertySpec(name="debug", value_type="enum", enum_values=("disabled", "enabled")),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="retries", value_type="integer"),
            BigipPropertySpec(
                name="service-type",
                value_type="enum",
                enum_values=(
                    "default",
                    "login",
                    "framed",
                    "callback-login",
                    "callback-framed",
                    "outbound",
                    "administrative",
                    "nas-prompt",
                    "authenticate-only",
                    "callback-nas-prompt",
                    "call-check",
                    "callback-administrative",
                ),
            ),
            BigipPropertySpec(
                name="servers", value_type="enum", allow_none=True, enum_values=("default", "none")
            ),
        ),
    )
