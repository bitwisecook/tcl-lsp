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
            "ltm_classification_urldb_file",
            module="ltm",
            object_types=("classification urldb-file",),
        ),
        header_types=(("ltm", "classification urldb-file"),),
        properties=(BigipPropertySpec(name="source-path", value_type="string"),),
    )
