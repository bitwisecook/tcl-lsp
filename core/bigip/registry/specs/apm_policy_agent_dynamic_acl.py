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
            "apm_policy_agent_dynamic_acl",
            module="apm",
            object_types=("policy agent dynamic-acl",),
        ),
        header_types=(("apm", "policy agent dynamic-acl"),),
        properties=(
            BigipPropertySpec(name="app-service", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="entries",
                value_type="list",
                allow_none=True,
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
            ),
            BigipPropertySpec(name="options", value_type="unknown"),
        ),
    )
