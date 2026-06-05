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
            "security_protocol_inspection_profile",
            module="security",
            object_types=("protocol-inspection profile",),
        ),
        header_types=(("security", "protocol-inspection profile"),),
        properties=(
            BigipPropertySpec(name="app-service", value_type="string"),
            BigipPropertySpec(name="auto-add-new-inspections", value_type="unknown"),
            BigipPropertySpec(name="auto-publish-suggestion", value_type="unknown"),
            BigipPropertySpec(name="avr-stat-collect", value_type="unknown"),
            BigipPropertySpec(name="common-config", value_type="string"),
            BigipPropertySpec(name="common-config-merge-type", value_type="string"),
            BigipPropertySpec(name="compliance-enable", value_type="unknown"),
            BigipPropertySpec(
                name="defaults-from",
                value_type="string",
                references=("security_protocol_inspection_profile",),
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="services", value_type="list", repeated=True),
            BigipPropertySpec(name="signature-enable", value_type="unknown"),
            BigipPropertySpec(name="staging-period", value_type="integer"),
        ),
    )
