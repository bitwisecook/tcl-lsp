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
            "security_bot_defense_signature_category",
            module="security",
            object_types=("bot-defense signature-category",),
        ),
        header_types=(("security", "bot-defense signature-category"),),
        properties=(BigipPropertySpec(name="class", value_type="string"),),
    )
