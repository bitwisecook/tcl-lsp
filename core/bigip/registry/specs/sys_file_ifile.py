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
            "sys_file_ifile",
            module="sys",
            object_types=("file ifile",),
        ),
        header_types=(("sys", "file ifile"),),
        properties=(BigipPropertySpec(name="source-path", value_type="string"),),
    )
