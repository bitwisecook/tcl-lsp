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
            "gtm_monitor_udp",
            module="gtm",
            object_types=("monitor udp",),
        ),
        header_types=(("gtm", "monitor udp"),),
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
                references=("gtm_monitor_udp",),
                default="udp",
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="destination", value_type="string", shape_kind="endpoint"),
            BigipPropertySpec(
                name="ignore-down-response",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(name="interval", value_type="integer", default="30 seconds"),
            BigipPropertySpec(name="probe-attempts", value_type="integer", default="3 attempts"),
            BigipPropertySpec(name="probe-interval", value_type="integer", default="1 second"),
            BigipPropertySpec(name="probe-timeout", value_type="integer", default="5 seconds"),
            BigipPropertySpec(name="recv", value_type="string", default="none"),
            BigipPropertySpec(
                name="reverse",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled, which specifies that the monitor does not operate in reverse mode",
            ),
            BigipPropertySpec(name="send", value_type="string", default='"default send string"'),
            BigipPropertySpec(name="timeout", value_type="integer", default="120 seconds"),
            BigipPropertySpec(
                name="transparent",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
        ),
    )
