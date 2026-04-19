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
            "sys_file_lwtunneltbl",
            module="sys",
            object_types=("file lwtunneltbl",),
        ),
        header_types=(("sys", "file lwtunneltbl"),),
        properties=(BigipPropertySpec(name="source-path", value_type="string"),),
    )
