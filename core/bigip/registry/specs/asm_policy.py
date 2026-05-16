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
            "asm_policy",
            module="asm",
            object_types=("policy",),
        ),
        header_types=(("asm", "policy"),),
        properties=(
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="blocking-mode",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="description",
                value_type="string",
                allow_none=True,
                usage_flags=frozenset(("optional",)),
            ),
            BigipPropertySpec(name="encoding", value_type="reference", allow_none=True),
            BigipPropertySpec(
                name="parent-policy",
                value_type="reference",
                usage_flags=frozenset(("optional",)),
            ),
            BigipPropertySpec(
                name="policy-builder",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(name="policy-template", value_type="reference"),
            BigipPropertySpec(
                name="policy-type",
                value_type="enum",
                enum_values=("parent", "security"),
            ),
        ),
    )
