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
            "ltm_policy",
            module="ltm",
            object_types=("policy",),
        ),
        header_types=(("ltm", "policy"),),
        properties=(
            BigipPropertySpec(name="copy-from", value_type="reference"),
            BigipPropertySpec(name="create-draft", value_type="unknown"),
            BigipPropertySpec(
                name="rules",
                value_type="list",
                references=("ltm_rule", "ltm_rule_profiler"),
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
                block=(
                    BigipPropertySpec(
                        name="ordinal",
                        value_type="list",
                        in_sections=("rules",),
                        list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
                    ),
                ),
            ),
            BigipPropertySpec(
                name="ordinal",
                value_type="list",
                in_sections=("rules",),
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
            ),
            BigipPropertySpec(
                name="strategy",
                value_type="enum",
                allow_none=True,
                enum_values=("STRING", "none"),
            ),
        ),
    )
