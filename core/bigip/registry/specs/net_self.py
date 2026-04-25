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
            "net_self",
            module="net",
            object_types=("self",),
        ),
        header_types=(("net", "self"),),
        properties=(
            BigipPropertySpec(
                name="address",
                value_type="string",
                pattern="^(?:\\\\d{1,3}(?:\\\\.\\\\d{1,3}){3})(?:/\\\\d{1,2})?$",
            ),
            BigipPropertySpec(
                name="address-source",
                value_type="reference",
                enum_values=("from-management", "from-user"),
                references=("auth_user",),
            ),
            BigipPropertySpec(
                name="allow-service",
                value_type="enum",
                allow_none=True,
                enum_values=("all", "default", "none"),
            ),
            BigipPropertySpec(name="description", value_type="string"),
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
            BigipPropertySpec(
                name="service-policy",
                value_type="reference",
                allow_none=True,
                references=("security_firewall_policy", "ltm_policy"),
            ),
            BigipPropertySpec(
                name="traffic-group",
                value_type="reference",
                allow_none=True,
                references=("cm_traffic_group",),
            ),
            BigipPropertySpec(name="vlan", value_type="reference", references=("net_vlan",)),
            BigipPropertySpec(name="reset-stats", value_type="list", repeated=True),
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
        ),
    )
