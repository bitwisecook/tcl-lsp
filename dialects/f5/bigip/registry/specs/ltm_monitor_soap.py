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
            "ltm_monitor_soap",
            module="ltm",
            object_types=("monitor soap",),
        ),
        header_types=(("ltm", "monitor soap"),),
        properties=(
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="debug",
                value_type="enum",
                enum_values=("no", "yes"),
                shape_kind="boolean",
                default="no",
            ),
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                references=("ltm_monitor_soap",),
                default="soap",
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="destination", value_type="string", shape_kind="endpoint"),
            BigipPropertySpec(
                name="expect-fault",
                value_type="enum",
                enum_values=("no", "yes"),
                shape_kind="boolean",
                default="no",
            ),
            BigipPropertySpec(name="interval", value_type="integer", default="5 seconds"),
            BigipPropertySpec(
                name="manual-resume",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(name="method", value_type="string"),
            BigipPropertySpec(
                name="namespace",
                value_type="reference",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="parameter-name",
                value_type="reference",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="parameter-type",
                value_type="enum",
                enum_values=("bool", "int", "long"),
                default="bool",
            ),
            BigipPropertySpec(name="parameter-value", value_type="integer", default="none"),
            BigipPropertySpec(name="password", value_type="unknown", default="none"),
            BigipPropertySpec(
                name="protocol",
                value_type="enum",
                enum_values=("http", "https"),
                default="http",
            ),
            BigipPropertySpec(
                name="return-type",
                value_type="enum",
                enum_values=("bool", "char", "double", "int", "long", "short"),
                default="bool",
            ),
            BigipPropertySpec(name="return-value", value_type="integer", default="none"),
            BigipPropertySpec(name="soap-action", value_type="string", default="the empty string"),
            BigipPropertySpec(name="time-until-up", value_type="integer", default="0 (zero)"),
            BigipPropertySpec(name="timeout", value_type="integer", default="16 seconds"),
            BigipPropertySpec(
                name="up-interval",
                value_type="integer",
                default="0 (zero), which specifies that the system uses the value of the interval option whether the resource is up or down",
            ),
            BigipPropertySpec(name="url-path", value_type="string", default="none"),
            BigipPropertySpec(
                name="username",
                value_type="reference",
                allow_none=True,
                default="none",
            ),
        ),
    )
