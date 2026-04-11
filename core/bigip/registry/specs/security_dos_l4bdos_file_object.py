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
            "security_dos_l4bdos_file_object",
            module="security",
            object_types=("dos l4bdos-file-object",),
        ),
        header_types=(("security", "dos l4bdos-file-object"),),
        properties=(
            BigipPropertySpec(name="context-name", value_type="string"),
            BigipPropertySpec(name="source-path", value_type="string"),
        ),
    )
