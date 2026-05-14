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
            "gtm_rule",
            module="gtm",
            object_types=("rule",),
        ),
        header_types=(("gtm", "rule"),),
        properties=(
            BigipPropertySpec(name="metadata", value_type="unknown"),
            BigipPropertySpec(
                name="persist",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(name="value", value_type="string"),
            BigipPropertySpec(
                name="when",
                value_type="string",
                usage_flags=frozenset(("read_only",)),
            ),
            BigipPropertySpec(name="}", value_type="string", usage_flags=frozenset(("read_only",))),
            BigipPropertySpec(
                name="host",
                value_type="string",
                usage_flags=frozenset(("read_only",)),
            ),
            BigipPropertySpec(
                name="log",
                value_type="string",
                usage_flags=frozenset(("read_only",)),
            ),
            BigipPropertySpec(name="#", value_type="string", usage_flags=frozenset(("read_only",))),
            BigipPropertySpec(
                name="\\[DNS::question",
                value_type="string",
                usage_flags=frozenset(("read_only",)),
            ),
        ),
    )
