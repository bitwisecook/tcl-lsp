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
            "ltm_persistence_persist_records",
            module="ltm",
            object_types=("persistence persist-records",),
        ),
        header_types=(("ltm", "persistence persist-records"),),
        properties=(
            BigipPropertySpec(name="client-addr", value_type="string"),
            BigipPropertySpec(name="key", value_type="string"),
            BigipPropertySpec(name="mode", value_type="string"),
            BigipPropertySpec(
                name="source-address",
                value_type="string",
                pattern="^(?:\\\\d{1,3}(?:\\\\.\\\\d{1,3}){3})(?:/\\\\d{1,2})?$",
            ),
            BigipPropertySpec(name="node-addr", value_type="string"),
            BigipPropertySpec(name="node-port", value_type="integer", min_value=0, max_value=65535),
            BigipPropertySpec(name="pool", value_type="reference", references=("ltm_pool",)),
            BigipPropertySpec(name="save-to-file", value_type="string"),
            BigipPropertySpec(name="virtual", value_type="string"),
        ),
    )
