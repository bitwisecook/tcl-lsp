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
            "auth_user",
            module="auth",
            object_types=("user",),
        ),
        header_types=(("auth", "user"),),
        properties=(
            BigipPropertySpec(name="description", value_type="list", repeated=True),
            BigipPropertySpec(
                name="partition-access",
                value_type="reference",
                repeated=True,
                references=("auth_partition",),
            ),
            BigipPropertySpec(name="password", value_type="string"),
            BigipPropertySpec(name="shell", value_type="string"),
            BigipPropertySpec(name="session-limit", value_type="integer"),
        ),
    )
