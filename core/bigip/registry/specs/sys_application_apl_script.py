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
            "sys_application_apl_script",
            module="sys",
            object_types=("application apl-script",),
        ),
        header_types=(("sys", "application apl-script"),),
        properties=(),
    )
