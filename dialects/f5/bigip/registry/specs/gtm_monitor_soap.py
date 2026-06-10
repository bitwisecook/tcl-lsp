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
            "gtm_monitor_soap",
            module="gtm",
            object_types=("monitor soap",),
        ),
        header_types=(("gtm", "monitor soap"),),
        properties=(
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
                references=("gtm_monitor_soap",),
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
            BigipPropertySpec(
                name="ignore-down-response",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(name="interval", value_type="integer", default="30 seconds"),
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
                default="bool",
            ),
            BigipPropertySpec(name="parameter-type", value_type="string", default="none"),
            BigipPropertySpec(name="parameter-value", value_type="integer", default="none"),
            BigipPropertySpec(name="password", value_type="unknown", default="none"),
            BigipPropertySpec(name="probe-timeout", value_type="integer", default="5 seconds"),
            BigipPropertySpec(name="protocol", value_type="unknown", default="none"),
            BigipPropertySpec(name="return-type", value_type="string", default="bool"),
            BigipPropertySpec(name="return-value", value_type="integer", default="none"),
            BigipPropertySpec(name="soap-action", value_type="string", default="the empty string"),
            BigipPropertySpec(name="timeout", value_type="integer", default="120 seconds"),
            BigipPropertySpec(name="url-path", value_type="string", default="none"),
            BigipPropertySpec(
                name="username",
                value_type="reference",
                allow_none=True,
                default="none",
            ),
        ),
    )
