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
            "apm_session",
            module="apm",
            object_types=("session",),
        ),
        header_types=(("apm", "session"),),
        properties=(),
    )
