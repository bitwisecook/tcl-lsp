from __future__ import annotations

from ..models import (
    BigipObjectKindSpec,
    BigipObjectSpec,
)
from ._base import register


@register
def register_spec() -> BigipObjectSpec:
    return BigipObjectSpec(
        kind_spec=BigipObjectKindSpec(
            "security_bot_defense_micro_service",
            module="security",
            object_types=("bot-defense micro-service",),
        ),
        header_types=(("security", "bot-defense micro-service"),),
    )
