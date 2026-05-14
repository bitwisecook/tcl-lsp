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
            "ltm_persistence_msrdp",
            module="ltm",
            object_types=("persistence msrdp",),
        ),
        header_types=(("ltm", "persistence msrdp"),),
        properties=(
            BigipPropertySpec(name="app-service", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                references=("ltm_persistence_msrdp",),
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="has-session-dir", value_type="enum", enum_values=("no", "yes")),
            BigipPropertySpec(
                name="match-across-pools",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="match-across-services",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="match-across-virtuals",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="mirror",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="override-connection-limit",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(name="timeout", value_type="integer"),
        ),
    )
