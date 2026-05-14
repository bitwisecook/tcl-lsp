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
            "util_ssh_keyswap",
            module="util",
            object_types=("ssh keyswap",),
        ),
        header_types=(("util", "ssh keyswap"),),
        properties=(),
    )
