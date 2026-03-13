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
            "auth_radius",
            module="auth",
            object_types=("radius",),
        ),
        header_types=(("auth", "radius"),),
        properties=(
            BigipPropertySpec(
                name="accounting-bug", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="client-id", value_type="integer", allow_none=True),
            BigipPropertySpec(name="debug", value_type="enum", enum_values=("disabled", "enabled")),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="retries", value_type="integer"),
            BigipPropertySpec(
                name="servers", value_type="enum", enum_values=("add", "delete", "replace-all-with")
            ),
            BigipPropertySpec(name="service-type", value_type="string"),
            BigipPropertySpec(name="callback-framed", value_type="string"),
            BigipPropertySpec(name="nas-prompt", value_type="string"),
            BigipPropertySpec(name="callback-nas-promit", value_type="string"),
        ),
    )
