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
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="csv-format",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(
                name="end-inbound-session",
                value_type="unknown",
                shape_kind="object",
                block=(
                    BigipPropertySpec(
                        name="action",
                        value_type="enum",
                        in_sections=("end-inbound-session",),
                        enum_values=("backup-allocation-only", "disabled", "enabled"),
                    ),
                ),
            ),
            BigipPropertySpec(
                name="action",
                value_type="enum",
                in_sections=("end-inbound-session",),
                enum_values=("backup-allocation-only", "disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="end-outbound-session",
                value_type="unknown",
                shape_kind="object",
                block=(
                    BigipPropertySpec(
                        name="action",
                        value_type="enum",
                        in_sections=("end-outbound-session",),
                        enum_values=("backup-allocation-only", "disabled", "enabled"),
                    ),
                    BigipPropertySpec(
                        name="elements",
                        value_type="list",
                        in_sections=("end-outbound-session",),
                        list_operators=frozenset(("add", "delete", "replace-all-with")),
                        usage_flags=frozenset(("optional",)),
                    ),
                ),
            ),
            BigipPropertySpec(
                name="action",
                value_type="enum",
                in_sections=("end-outbound-session",),
                enum_values=("backup-allocation-only", "disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="elements",
                value_type="list",
                in_sections=("end-outbound-session",),
                list_operators=frozenset(("add", "delete", "replace-all-with")),
                usage_flags=frozenset(("optional",)),
                block=(
                    BigipPropertySpec(
                        name="destination",
                        value_type="unknown",
                        in_sections=("end-outbound-session", "elements"),
                    ),
                ),
            ),
            BigipPropertySpec(
                name="destination",
                value_type="unknown",
                in_sections=("end-outbound-session", "elements"),
            ),
            BigipPropertySpec(
                name="errors",
                value_type="list",
                block=(
                    BigipPropertySpec(
                        name="action",
                        value_type="enum",
                        in_sections=("errors",),
                        enum_values=("disabled", "enabled"),
                        shape_kind="boolean",
                    ),
                ),
            ),
            BigipPropertySpec(
                name="action",
                value_type="enum",
                in_sections=("errors",),
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="quota-exceeded",
                value_type="unknown",
                shape_kind="object",
                block=(
                    BigipPropertySpec(
                        name="action",
                        value_type="enum",
                        in_sections=("quota-exceeded",),
                        enum_values=("disabled", "enabled"),
                        shape_kind="boolean",
                    ),
                ),
            ),
            BigipPropertySpec(
                name="action",
                value_type="enum",
                in_sections=("quota-exceeded",),
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="start-inbound-session",
                value_type="unknown",
                shape_kind="object",
                block=(
                    BigipPropertySpec(
                        name="action",
                        value_type="enum",
                        in_sections=("start-inbound-session",),
                        enum_values=("backup-allocation-only", "disabled", "enabled"),
                    ),
                ),
            ),
            BigipPropertySpec(
                name="action",
                value_type="enum",
                in_sections=("start-inbound-session",),
                enum_values=("backup-allocation-only", "disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="start-outbound-session",
                value_type="unknown",
                shape_kind="object",
                block=(
                    BigipPropertySpec(
                        name="action",
                        value_type="enum",
                        in_sections=("start-outbound-session",),
                        enum_values=("backup-allocation-only", "disabled", "enabled"),
                    ),
                    BigipPropertySpec(
                        name="elements",
                        value_type="list",
                        in_sections=("start-outbound-session",),
                        list_operators=frozenset(("add", "delete", "replace-all-with")),
                        usage_flags=frozenset(("optional",)),
                    ),
                ),
            ),
            BigipPropertySpec(
                name="action",
                value_type="enum",
                in_sections=("start-outbound-session",),
                enum_values=("backup-allocation-only", "disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="elements",
                value_type="list",
                in_sections=("start-outbound-session",),
                list_operators=frozenset(("add", "delete", "replace-all-with")),
                usage_flags=frozenset(("optional",)),
                block=(
                    BigipPropertySpec(
                        name="destination",
                        value_type="unknown",
                        in_sections=("start-outbound-session", "elements"),
                    ),
                ),
            ),
            BigipPropertySpec(
                name="destination",
                value_type="unknown",
                in_sections=("start-outbound-session", "elements"),
            ),
            BigipPropertySpec(
                name="subscriber-id",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
            ),
        ),
    )
