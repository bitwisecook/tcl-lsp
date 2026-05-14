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
            BigipPropertySpec(name="description", value_type="unknown", repeated=True),
            BigipPropertySpec(
                name="partition-access",
                value_type="list",
                references=("auth_partition",),
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
            ),
            BigipPropertySpec(name="password", value_type="unknown"),
            BigipPropertySpec(name="prompt-for-password", value_type="unknown"),
            BigipPropertySpec(name="session-limit", value_type="integer"),
            BigipPropertySpec(name="shell", value_type="reference"),
        ),
    )
