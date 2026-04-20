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
            "security_cloud_services_connector",
            module="security",
            object_types=("cloud-services connector",),
        ),
        header_types=(("security", "cloud-services connector"),),
        properties=(
            BigipPropertySpec(name="activation-time", value_type="string"),
            BigipPropertySpec(name="activation-note", value_type="string"),
            BigipPropertySpec(name="expiration-time", value_type="string"),
            BigipPropertySpec(name="deployment-id", value_type="integer"),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="params", value_type="string"),
            BigipPropertySpec(name="control-url", value_type="string"),
            BigipPropertySpec(name="control-token", value_type="string"),
            BigipPropertySpec(name="control-key", value_type="string"),
            BigipPropertySpec(name="clientside-url", value_type="string"),
            BigipPropertySpec(name="clientside-token", value_type="string"),
            BigipPropertySpec(name="clientside-key", value_type="string"),
            BigipPropertySpec(name="services", value_type="string"),
            BigipPropertySpec(
                name="centralized-device-id", value_type="integer", in_sections=("services",)
            ),
        ),
    )
