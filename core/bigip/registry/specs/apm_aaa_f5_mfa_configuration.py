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
            "apm_aaa_f5_mfa_configuration",
            module="apm",
            object_types=("aaa f5-mfa-configuration",),
        ),
        header_types=(("apm", "aaa f5-mfa-configuration"),),
        properties=(
            BigipPropertySpec(name="app-service", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="f5-service-connector",
                value_type="reference",
                references=("apm_aaa_f5_service_connector",),
            ),
            BigipPropertySpec(
                name="max-mobile-devices-per-user",
                value_type="integer",
                allow_none=True,
            ),
            BigipPropertySpec(
                name="permitted-devices-types",
                value_type="list",
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
            ),
            BigipPropertySpec(
                name="registration-sms-template",
                value_type="string",
                allow_none=True,
            ),
            BigipPropertySpec(
                name="require-biometric",
                value_type="enum",
                allow_none=True,
                enum_values=("false", "true"),
            ),
        ),
    )
