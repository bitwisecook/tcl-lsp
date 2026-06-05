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
            BigipPropertySpec(name="app-service", value_type="reference", default="none"),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="password", value_type="string"),
            BigipPropertySpec(name="poll-interval", value_type="integer"),
            BigipPropertySpec(name="priority", value_type="enum", enum_values=("high", "low")),
            BigipPropertySpec(name="url", value_type="string"),
            BigipPropertySpec(name="user", value_type="string"),
        ),
    )
