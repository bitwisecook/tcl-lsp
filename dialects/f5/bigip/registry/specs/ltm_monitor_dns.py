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
            "ltm_monitor_dns",
            module="ltm",
            object_types=("monitor dns",),
        ),
        header_types=(("ltm", "monitor dns"),),
        properties=(
            BigipPropertySpec(
                name="accept-rcode",
                value_type="enum",
                enum_values=("anything", "no-error"),
            ),
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
                name="answer-contains",
                value_type="enum",
                enum_values=("any-type", "anything", "query-type"),
            ),
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                references=("ltm_monitor_dns",),
                default="dns",
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="destination", value_type="string", shape_kind="endpoint"),
            BigipPropertySpec(name="interval", value_type="integer", default="5 seconds"),
            BigipPropertySpec(
                name="manual-resume",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(name="qname", value_type="string", default="Enter a query name"),
            BigipPropertySpec(
                name="qtype",
                value_type="enum",
                enum_values=("a", "aaaa"),
                default="a",
            ),
            BigipPropertySpec(name="recv", value_type="string", default="none"),
            BigipPropertySpec(
                name="reverse",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled, which specifies that the monitor does not operate in reverse mode",
            ),
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
        ),
    )
