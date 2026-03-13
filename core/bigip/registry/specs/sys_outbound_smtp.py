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
            "sys_outbound_smtp",
            module="sys",
            object_types=("outbound-smtp",),
        ),
        header_types=(("sys", "outbound-smtp"),),
        properties=(
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="from-line-override", value_type="string"),
            BigipPropertySpec(name="mailhub", value_type="string"),
            BigipPropertySpec(name="rewrite-domain", value_type="string"),
        ),
    )
