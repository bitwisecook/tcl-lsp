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
            "security_packet_filter_default_rules",
            module="security",
            object_types=("packet-filter default-rules",),
        ),
        header_types=(("security", "packet-filter default-rules"),),
        properties=(
            BigipPropertySpec(
                name="policy", value_type="reference", allow_none=True, references=("ltm_policy",)
            ),
            BigipPropertySpec(name="security", value_type="reference", references=("ltm_rule",)),
            BigipPropertySpec(
                name="policy",
                value_type="reference",
                in_sections=("security",),
                references=("ltm_policy",),
            ),
        ),
    )
