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
            "ltm_alg_log_profile",
            module="ltm",
            object_types=("alg-log-profile",),
        ),
        header_types=(("ltm", "alg-log-profile"),),
        properties=(
            BigipPropertySpec(
                name="csv-format", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="start-control-channel", value_type="string"),
            BigipPropertySpec(
                name="action",
                value_type="enum",
                in_sections=("start-control-channel",),
                enum_values=("disabled", "enabled", "backup-allocation-only"),
            ),
            BigipPropertySpec(
                name="elements",
                value_type="enum",
                in_sections=("start-control-channel",),
                enum_values=("add", "delete", "replace-all-with"),
            ),
            BigipPropertySpec(name="end-control-channel", value_type="string"),
            BigipPropertySpec(
                name="action",
                value_type="enum",
                in_sections=("end-control-channel",),
                enum_values=("disabled", "enabled", "backup-allocation-only"),
            ),
            BigipPropertySpec(
                name="elements",
                value_type="enum",
                in_sections=("end-control-channel",),
                enum_values=("add", "delete", "replace-all-with"),
            ),
            BigipPropertySpec(name="start-data-channel", value_type="string"),
            BigipPropertySpec(
                name="action",
                value_type="enum",
                in_sections=("start-data-channel",),
                enum_values=("disabled", "enabled", "backup-allocation-only"),
            ),
            BigipPropertySpec(
                name="elements",
                value_type="enum",
                in_sections=("start-data-channel",),
                enum_values=("add", "delete", "replace-all-with"),
            ),
            BigipPropertySpec(name="end-data-channel", value_type="string"),
            BigipPropertySpec(
                name="action",
                value_type="enum",
                in_sections=("end-data-channel",),
                enum_values=("disabled", "enabled", "backup-allocation-only"),
            ),
            BigipPropertySpec(
                name="elements",
                value_type="enum",
                in_sections=("end-data-channel",),
                enum_values=("add", "delete", "replace-all-with"),
            ),
            BigipPropertySpec(name="inbound-transaction", value_type="string"),
            BigipPropertySpec(
                name="action",
                value_type="enum",
                in_sections=("inbound-transaction",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(name="reset-stats", value_type="string"),
        ),
    )
