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
            "ltm_message_routing_sip_profile_session",
            module="ltm",
            object_types=("message-routing sip profile session",),
        ),
        header_types=(("ltm", "message-routing sip profile session"),),
        properties=(
            BigipPropertySpec(
                name="honor-route-mode", value_type="enum", enum_values=("loose", "strict")
            ),
            BigipPropertySpec(name="record-route-mode", value_type="string"),
            BigipPropertySpec(
                name="service-port", value_type="integer", min_value=0, max_value=65535
            ),
            BigipPropertySpec(
                name="allow-unknown-methods", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="custom-via", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="defaults-from", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="do-not-connect-back", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(
                name="generate-response-on-failure",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="honor-via", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(
                name="insert-record-route-header",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="honor-route-header", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(
                name="insert-via-header", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(
                name="maintenance-mode", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(
                name="loop-detection", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(
                name="loop-detection-mode", value_type="enum", enum_values=("loose", "strict")
            ),
            BigipPropertySpec(
                name="max-forwards-check", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="max-msg-header-count", value_type="integer"),
            BigipPropertySpec(name="max-msg-header-size", value_type="integer"),
            BigipPropertySpec(name="max-msg-size", value_type="integer"),
            BigipPropertySpec(
                name="passthru-mode", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(
                name="persistence",
                value_type="reference",
                references=(
                    "ltm_persistence_cookie",
                    "ltm_persistence_dest_addr",
                    "ltm_persistence_global_settings",
                    "ltm_persistence_hash",
                    "ltm_persistence_host",
                    "ltm_persistence_msrdp",
                    "ltm_persistence_persist_records",
                    "ltm_persistence_sip",
                    "ltm_persistence_source_addr",
                    "ltm_persistence_ssl",
                    "ltm_persistence_universal",
                ),
            ),
            BigipPropertySpec(
                name="persist-key",
                value_type="enum",
                in_sections=("persistence",),
                enum_values=("call-id", "custom", "src-addr"),
            ),
            BigipPropertySpec(
                name="persist-timeout", value_type="integer", in_sections=("persistence",)
            ),
            BigipPropertySpec(
                name="persist-type",
                value_type="enum",
                in_sections=("persistence",),
                allow_none=True,
                enum_values=("session", "none"),
            ),
            BigipPropertySpec(
                name="enable-sip-firewall", value_type="enum", enum_values=("no", "yes")
            ),
            BigipPropertySpec(name="reset-stats", value_type="string"),
        ),
    )
