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
            "ltm_tacdb_customdb_file",
            module="ltm",
            object_types=("tacdb customdb-file",),
        ),
        header_types=(("ltm", "tacdb customdb-file"),),
        properties=(BigipPropertySpec(name="source-path", value_type="string"),),
    )
