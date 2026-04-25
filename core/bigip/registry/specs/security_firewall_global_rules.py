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
            "security_firewall_global_rules",
            module="security",
            object_types=("firewall global-rules",),
        ),
        header_types=(("security", "firewall global-rules"),),
        properties=(
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="enforced-policy", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="staged-policy", value_type="boolean", allow_none=True),
            BigipPropertySpec(
                name="service-policy",
                value_type="reference",
                allow_none=True,
                references=("security_firewall_policy", "ltm_policy"),
            ),
            BigipPropertySpec(name="reset-stats", value_type="reference", references=("ltm_rule",)),
            BigipPropertySpec(
                name="enforced-policy-rules",
                value_type="reference",
                repeated=True,
                references=("ltm_rule",),
            ),
            BigipPropertySpec(
                name="staged-policy-rules",
                value_type="reference",
                repeated=True,
                references=("ltm_rule",),
            ),
            BigipPropertySpec(name="security", value_type="reference", references=("ltm_rule",)),
            BigipPropertySpec(
                name="enforced-policy", value_type="string", in_sections=("security",)
            ),
        ),
    )
