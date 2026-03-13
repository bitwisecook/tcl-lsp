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
            "sys_scriptd",
            module="sys",
            object_types=("scriptd",),
        ),
        header_types=(("sys", "scriptd"),),
        properties=(
            BigipPropertySpec(
                name="log-level",
                value_type="enum",
                enum_values=("alert", "crit", "debug", "emerg", "err", "info", "notice", "warn"),
            ),
            BigipPropertySpec(name="max-script-run-time", value_type="string"),
        ),
    )
