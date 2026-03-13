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
            "sys_file_rewrite_rule",
            module="sys",
            object_types=("file rewrite-rule",),
        ),
        header_types=(("sys", "file rewrite-rule"),),
        properties=(BigipPropertySpec(name="local-path", value_type="string"),),
    )
