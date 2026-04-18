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
            "net_route_domain",
            module="net",
            object_types=("route-domain",),
        ),
        header_types=(("net", "route-domain"),),
        properties=(
            BigipPropertySpec(name="id", value_type="integer"),
            BigipPropertySpec(name="bwc-policy", value_type="string"),
            BigipPropertySpec(name="connection-limit", value_type="integer"),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="flow-eviction-policy", value_type="boolean", allow_none=True),
            BigipPropertySpec(
                name="fw-enforced-policy",
                value_type="reference",
                allow_none=True,
                references=("security_firewall_policy", "ltm_policy"),
            ),
            BigipPropertySpec(
                name="fw-staged-policy",
                value_type="reference",
                allow_none=True,
                references=("security_firewall_policy", "ltm_policy"),
            ),
            BigipPropertySpec(name="parent", value_type="boolean", allow_none=True),
            BigipPropertySpec(
                name="security-nat-policy",
                value_type="reference",
                allow_none=True,
                references=("security_nat_policy",),
            ),
            BigipPropertySpec(
                name="service-policy",
                value_type="reference",
                allow_none=True,
                references=("security_firewall_policy", "ltm_policy"),
            ),
            BigipPropertySpec(
                name="strict", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="reset-stats", value_type="string"),
            BigipPropertySpec(
                name="fw-enforced-policy-rules",
                value_type="reference",
                repeated=True,
                references=("ltm_rule",),
            ),
            BigipPropertySpec(
                name="fw-staged-policy-rules",
                value_type="reference",
                repeated=True,
                references=("ltm_rule",),
            ),
            BigipPropertySpec(
                name="security-nat-rules",
                value_type="reference",
                repeated=True,
                references=("ltm_rule",),
            ),
        ),
    )
