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
            "ltm_tacdb_customdb",
            module="ltm",
            object_types=("tacdb customdb",),
        ),
        header_types=(("ltm", "tacdb customdb"),),
        properties=(
            BigipPropertySpec(name="url", value_type="string"),
            BigipPropertySpec(name="poll-interval", value_type="integer"),
            BigipPropertySpec(name="user", value_type="reference", references=("auth_user",)),
            BigipPropertySpec(name="password", value_type="string"),
            BigipPropertySpec(name="priority", value_type="enum", enum_values=("high", "low")),
            BigipPropertySpec(name="description", value_type="string"),
        ),
    )
