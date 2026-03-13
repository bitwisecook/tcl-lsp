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
                name="accept-rcode", value_type="enum", enum_values=("no-error", "anything")
            ),
            BigipPropertySpec(
                name="adaptive", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(
                name="adaptive-divergence-type",
                value_type="enum",
                enum_values=("relative", "absolute"),
            ),
            BigipPropertySpec(name="adaptive-divergence-value", value_type="integer"),
            BigipPropertySpec(name="adaptive-limit", value_type="integer"),
            BigipPropertySpec(name="adaptive-sampling-timespan", value_type="integer"),
            BigipPropertySpec(
                name="answer-contains",
                value_type="enum",
                enum_values=("query-type", "any-type", "anything"),
            ),
            BigipPropertySpec(
                name="defaults-from", value_type="reference", references=("ltm_monitor_dns",)
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="destination", value_type="string"),
            BigipPropertySpec(name="interval", value_type="integer"),
            BigipPropertySpec(
                name="manual-resume", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(name="qname", value_type="string"),
            BigipPropertySpec(name="qtype", value_type="enum", enum_values=("a", "aaaa")),
            BigipPropertySpec(name="recv", value_type="boolean", allow_none=True),
            BigipPropertySpec(
                name="reverse", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(name="time-until-up", value_type="integer"),
            BigipPropertySpec(name="timeout", value_type="integer"),
            BigipPropertySpec(
                name="transparent", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="up-interval", value_type="integer"),
            BigipPropertySpec(name="stop", value_type="string"),
        ),
    )
