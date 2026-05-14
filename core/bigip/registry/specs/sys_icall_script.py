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
            "sys_icall_script",
            module="sys",
            object_types=("icall script",),
        ),
        header_types=(("sys", "icall script"),),
        properties=(
            BigipPropertySpec(name="definition", value_type="unknown"),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="events",
                value_type="list",
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
                block=(
                    BigipPropertySpec(
                        name="contexts",
                        value_type="list",
                        in_sections=("events",),
                        list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
                    ),
                ),
            ),
            BigipPropertySpec(
                name="contexts",
                value_type="list",
                in_sections=("events",),
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
            ),
        ),
    )
