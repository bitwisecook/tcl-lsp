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
            "apm_aaa_okta_connector",
            module="apm",
            object_types=("aaa okta-connector",),
        ),
        header_types=(("apm", "aaa okta-connector"),),
        properties=(
            BigipPropertySpec(name="domain", value_type="string", required=True),
            BigipPropertySpec(name="token", value_type="string", required=True),
            BigipPropertySpec(
                name="transport",
                value_type="reference",
                required=True,
                references=(
                    "apm_aaa_http_connector_transport",
                    "ltm_message_routing_diameter_transport_config",
                    "ltm_message_routing_generic_transport_config",
                    "ltm_message_routing_mqtt_transport_config",
                    "ltm_message_routing_sip_transport_config",
                ),
            ),
        ),
    )
