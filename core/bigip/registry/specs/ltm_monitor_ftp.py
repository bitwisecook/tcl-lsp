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
            "ltm_monitor_ftp",
            module="ltm",
            object_types=("monitor ftp",),
        ),
        header_types=(("ltm", "monitor ftp"),),
        properties=(
            BigipPropertySpec(name="app-service", value_type="string", allow_none=True),
            BigipPropertySpec(name="debug", value_type="enum", enum_values=("no", "yes")),
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                references=("ltm_monitor_ftp",),
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="destination", value_type="string"),
            BigipPropertySpec(name="filename", value_type="unknown", allow_none=True),
            BigipPropertySpec(name="interval", value_type="integer"),
            BigipPropertySpec(
                name="manual-resume",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(name="mode", value_type="enum", enum_values=("passive",)),
            BigipPropertySpec(name="password", value_type="unknown"),
            BigipPropertySpec(name="time-until-up", value_type="integer"),
            BigipPropertySpec(name="timeout", value_type="integer"),
            BigipPropertySpec(name="up-interval", value_type="integer"),
            BigipPropertySpec(name="username", value_type="reference"),
        ),
    )
