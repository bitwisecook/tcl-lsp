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
            "apm_oauth_db_instance",
            module="apm",
            object_types=("oauth db-instance",),
        ),
        header_types=(("apm", "oauth db-instance"),),
        properties=(
            BigipPropertySpec(name="app-service", value_type="string", allow_none=True),
            BigipPropertySpec(name="description", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="purge-frequency",
                value_type="enum",
                enum_values=("daily", "hourly", "monthly", "never", "weekly"),
            ),
            BigipPropertySpec(name="purge-now", value_type="unknown"),
            BigipPropertySpec(name="purge-time", value_type="string"),
        ),
    )
