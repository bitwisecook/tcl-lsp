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
            "ltm_lsn_log_profile",
            module="ltm",
            object_types=("lsn-log-profile",),
        ),
        header_types=(("ltm", "lsn-log-profile"),),
        properties=(
            BigipPropertySpec(
                name="csv-format", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="start-outbound-session", value_type="string"),
            BigipPropertySpec(
                name="action",
                value_type="enum",
                in_sections=("start-outbound-session",),
                enum_values=("disabled", "enabled", "backup-allocation-only"),
            ),
            BigipPropertySpec(
                name="elements",
                value_type="enum",
                in_sections=("start-outbound-session",),
                enum_values=("add", "delete", "replace-all-with"),
            ),
            BigipPropertySpec(name="end-outbound-session", value_type="string"),
            BigipPropertySpec(
                name="action",
                value_type="enum",
                in_sections=("end-outbound-session",),
                enum_values=("disabled", "enabled", "backup-allocation-only"),
            ),
            BigipPropertySpec(
                name="elements",
                value_type="enum",
                in_sections=("end-outbound-session",),
                enum_values=("add", "delete", "replace-all-with"),
            ),
            BigipPropertySpec(name="start-inbound-session", value_type="string"),
            BigipPropertySpec(
                name="action",
                value_type="enum",
                in_sections=("start-inbound-session",),
                enum_values=("disabled", "enabled", "backup-allocation-only"),
            ),
            BigipPropertySpec(name="end-inbound-session", value_type="string"),
            BigipPropertySpec(
                name="action",
                value_type="enum",
                in_sections=("end-inbound-session",),
                enum_values=("disabled", "enabled", "backup-allocation-only"),
            ),
            BigipPropertySpec(name="quota-exceeded", value_type="string"),
            BigipPropertySpec(
                name="action",
                value_type="enum",
                in_sections=("quota-exceeded",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(name="errors", value_type="string"),
            BigipPropertySpec(
                name="action",
                value_type="enum",
                in_sections=("errors",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="subscriber-id", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="reset-stats", value_type="string"),
        ),
    )
