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
            "sys_file_data_group",
            module="sys",
            object_types=("file data-group",),
        ),
        header_types=(("sys", "file data-group"),),
        properties=(
            BigipPropertySpec(name="data-group-description", value_type="string"),
            BigipPropertySpec(name="data-group-name", value_type="string"),
            BigipPropertySpec(name="separator", value_type="string"),
            BigipPropertySpec(name="source-path", value_type="string"),
            BigipPropertySpec(name="type", value_type="integer"),
        ),
    )
