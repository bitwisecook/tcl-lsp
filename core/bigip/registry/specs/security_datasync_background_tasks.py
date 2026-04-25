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
            "security_datasync_background_tasks",
            module="security",
            object_types=("datasync background-tasks",),
        ),
        header_types=(("security", "datasync background-tasks"),),
        properties=(BigipPropertySpec(name="daily-start-time", value_type="string"),),
    )
