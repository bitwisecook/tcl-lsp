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
            "ltm_monitor_https",
            module="ltm",
            object_types=("monitor https",),
        ),
        header_types=(("ltm", "monitor https"),),
        properties=(
            BigipPropertySpec(
                name="adaptive",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="adaptive-divergence-type",
                value_type="enum",
                enum_values=("absolute", "relative"),
            ),
            BigipPropertySpec(name="adaptive-divergence-value", value_type="integer"),
            BigipPropertySpec(name="adaptive-limit", value_type="integer"),
            BigipPropertySpec(name="adaptive-sampling-timespan", value_type="integer"),
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="cert",
                value_type="unknown",
                allow_none=True,
                default="none",
                usage_flags=frozenset(("deprecated",)),
            ),
            BigipPropertySpec(
                name="cipherlist",
                value_type="string",
                usage_flags=frozenset(("deprecated",)),
            ),
            BigipPropertySpec(
                name="compatibility",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="enabled",
                usage_flags=frozenset(("deprecated",)),
            ),
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                references=("ltm_monitor_https",),
                default="https",
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="destination",
                value_type="string",
                shape_kind="endpoint",
                default="*:*",
            ),
            BigipPropertySpec(name="interval", value_type="integer", default="5 seconds"),
            BigipPropertySpec(name="ip-dscp", value_type="integer", default="zero"),
            BigipPropertySpec(
                name="key",
                value_type="unknown",
                allow_none=True,
                default="none",
                usage_flags=frozenset(("deprecated",)),
            ),
            BigipPropertySpec(
                name="manual-resume",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(name="password", value_type="unknown", default="none"),
            BigipPropertySpec(name="recv", value_type="string", default="none"),
            BigipPropertySpec(name="recv-disable", value_type="string", default="none"),
            BigipPropertySpec(
                name="reverse",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled, which specifies that the monitor does not operate in reverse mode",
            ),
            BigipPropertySpec(name="send", value_type="string"),
            BigipPropertySpec(name="ssl-profile", value_type="unknown", allow_none=True),
            BigipPropertySpec(name="time-until-up", value_type="integer", default="0 (zero)"),
            BigipPropertySpec(name="timeout", value_type="integer", default="16 seconds"),
            BigipPropertySpec(
                name="transparent",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(
                name="up-interval",
                value_type="integer",
                default="0 (zero), which specifies that the system uses the value of the interval option whether the resource is up or down",
            ),
            BigipPropertySpec(
                name="username",
                value_type="reference",
                allow_none=True,
                default="none",
            ),
        ),
    )
