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
            "util_finalize_custom_ami",
            module="util",
            object_types=("finalize custom ami",),
        ),
        header_types=(("util", "finalize custom ami"),),
        properties=(),
    )
