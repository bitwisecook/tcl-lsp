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
            BigipPropertySpec(name="debug", value_type="enum", enum_values=("no", "yes")),
            BigipPropertySpec(
                name="defaults-from", value_type="reference", references=("ltm_monitor_soap",)
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="destination", value_type="string"),
            BigipPropertySpec(name="expect-fault", value_type="enum", enum_values=("no", "yes")),
            BigipPropertySpec(name="interval", value_type="integer"),
            BigipPropertySpec(
                name="manual-resume", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(name="method", value_type="string"),
            BigipPropertySpec(name="namespace", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="parameter-name", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="parameter-type", value_type="string"),
            BigipPropertySpec(name="parameter-value", value_type="integer", allow_none=True),
            BigipPropertySpec(
                name="password",
                value_type="enum",
                allow_none=True,
                enum_values=("none", "password"),
            ),
            BigipPropertySpec(name="protocol", value_type="enum", enum_values=("http", "https")),
            BigipPropertySpec(name="return-type", value_type="string"),
            BigipPropertySpec(name="return-value", value_type="integer", allow_none=True),
            BigipPropertySpec(name="soap-action", value_type="string"),
            BigipPropertySpec(name="time-until-up", value_type="integer"),
            BigipPropertySpec(name="timeout", value_type="integer"),
            BigipPropertySpec(name="up-interval", value_type="integer"),
            BigipPropertySpec(name="url-path", value_type="boolean", allow_none=True),
            BigipPropertySpec(
                name="username", value_type="reference", allow_none=True, references=("auth_user",)
            ),
            BigipPropertySpec(name="stop", value_type="string"),
        ),
    )
