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
            "security_log_remote_format",
            module="security",
            object_types=("log remote-format",),
        ),
        header_types=(("security", "log remote-format"),),
    )
