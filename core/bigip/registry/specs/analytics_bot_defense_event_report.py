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
            "analytics_bot_defense_event_report",
            module="analytics",
            object_types=("bot-defense-event report",),
        ),
        header_types=(("analytics", "bot-defense-event report"),),
        properties=(),
    )
