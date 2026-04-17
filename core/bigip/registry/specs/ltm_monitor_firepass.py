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
            "ltm_monitor_firepass",
            module="ltm",
            object_types=("monitor firepass",),
        ),
        header_types=(("ltm", "monitor firepass"),),
        properties=(
            BigipPropertySpec(name="cipherlist", value_type="string"),
            BigipPropertySpec(name="concurrency-limit", value_type="integer"),
            BigipPropertySpec(
                name="defaults-from", value_type="reference", references=("ltm_monitor_firepass",)
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="destination", value_type="string"),
            BigipPropertySpec(name="interval", value_type="integer"),
            BigipPropertySpec(name="max-load-average", value_type="integer"),
            BigipPropertySpec(name="password", value_type="string"),
            BigipPropertySpec(name="time-until-up", value_type="integer"),
            BigipPropertySpec(name="timeout", value_type="integer"),
            BigipPropertySpec(name="up-interval", value_type="integer"),
            BigipPropertySpec(
                name="username", value_type="reference", allow_none=True, references=("auth_user",)
            ),
            BigipPropertySpec(name="stop", value_type="string"),
        ),
    )
