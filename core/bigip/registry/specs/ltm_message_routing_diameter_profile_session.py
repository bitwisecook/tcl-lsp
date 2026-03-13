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
            "ltm_message_routing_diameter_profile_session",
            module="ltm",
            object_types=("message-routing diameter profile session",),
        ),
        header_types=(("ltm", "message-routing diameter profile session"),),
        properties=(
            BigipPropertySpec(name="acct-application-id", value_type="integer"),
            BigipPropertySpec(
                name="array-acct-application-id", value_type="integer", allow_none=True
            ),
            BigipPropertySpec(
                name="array-auth-application-id", value_type="integer", allow_none=True
            ),
            BigipPropertySpec(
                name="array-retransmission-result-codes", value_type="integer", allow_none=True
            ),
            BigipPropertySpec(name="auth-application-id", value_type="integer"),
            BigipPropertySpec(name="defaults-from", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="dest-host-rewrite", value_type="string"),
            BigipPropertySpec(name="dest-realm-rewrite", value_type="string"),
            BigipPropertySpec(
                name="disconnect-peer-action",
                value_type="enum",
                allow_none=True,
                enum_values=("disable", "force-offline", "none"),
            ),
            BigipPropertySpec(
                name="dynamic-route-insertion",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="dynamic-route-lookup", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="dynamic-route-timeout", value_type="integer"),
            BigipPropertySpec(
                name="discard-unroutable", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="egress-critical-message-rate-limit", value_type="integer"),
            BigipPropertySpec(name="egress-major-message-rate-limit", value_type="integer"),
            BigipPropertySpec(name="handshake-timeout", value_type="integer"),
            BigipPropertySpec(
                name="host-ip-address",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                pattern="^(?:\\\\d{1,3}(?:\\\\.\\\\d{1,3}){3})(?:/\\\\d{1,2})?$",
            ),
            BigipPropertySpec(name="ingress-critical-message-rate-limit", value_type="integer"),
            BigipPropertySpec(name="ingress-major-message-rate-limit", value_type="integer"),
            BigipPropertySpec(
                name="loop-detection", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="max-message-size", value_type="integer"),
            BigipPropertySpec(name="max-retransmissions", value_type="integer"),
            BigipPropertySpec(name="max-watchdog-failures", value_type="integer"),
            BigipPropertySpec(name="origin-host", value_type="string"),
            BigipPropertySpec(name="origin-host-rewrite", value_type="string"),
            BigipPropertySpec(name="origin-realm", value_type="string"),
            BigipPropertySpec(name="origin-realm-rewrite", value_type="string"),
            BigipPropertySpec(name="persist-avp", value_type="string"),
            BigipPropertySpec(name="persist-timeout", value_type="integer"),
            BigipPropertySpec(
                name="persist-type",
                value_type="enum",
                allow_none=True,
                enum_values=("avp", "custom", "none"),
            ),
            BigipPropertySpec(name="product-name", value_type="string"),
            BigipPropertySpec(
                name="reset-on-timeout", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(
                name="respond-unroutable", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(
                name="retransmission-action",
                value_type="enum",
                enum_values=("disabled", "busy", "unable", "retransmit", "retransmit-alternate"),
            ),
            BigipPropertySpec(name="retransmission-queue-limit-low", value_type="integer"),
            BigipPropertySpec(name="retransmission-queue-limit-high", value_type="integer"),
            BigipPropertySpec(name="retransmission-queue-max-bytes", value_type="integer"),
            BigipPropertySpec(name="retransmission-queue-max-messages", value_type="integer"),
            BigipPropertySpec(name="retransmission-timeout", value_type="integer"),
            BigipPropertySpec(
                name="route-unconfigured-peers",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(name="vendor-id", value_type="integer"),
            BigipPropertySpec(name="vendor-specific-vendor-id", value_type="integer"),
            BigipPropertySpec(name="vendor-specific-acct-application-id", value_type="integer"),
            BigipPropertySpec(name="vendor-specific-auth-application-id", value_type="integer"),
            BigipPropertySpec(name="watchdog-timeout", value_type="integer"),
            BigipPropertySpec(name="reset-stats", value_type="string"),
        ),
    )
