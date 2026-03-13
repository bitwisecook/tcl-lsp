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
            "ltm_profile_tcp",
            module="ltm",
            object_types=("profile tcp",),
        ),
        header_types=(("ltm", "profile tcp"),),
        properties=(
            BigipPropertySpec(name="abc", value_type="enum", enum_values=("disabled", "enabled")),
            BigipPropertySpec(
                name="ack-on-push", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(
                name="auto-proxy-buffer-size",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="auto-receive-window-size",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="auto-send-buffer-size", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="close-wait-timeout", value_type="integer"),
            BigipPropertySpec(
                name="cmetrics-cache", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="cmetrics-cache-timeout", value_type="integer"),
            BigipPropertySpec(name="congestion-control", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="vegas", value_type="boolean"),
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                allow_none=True,
                references=("ltm_profile_tcp",),
            ),
            BigipPropertySpec(
                name="deferred-accept", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(
                name="delay-window-control", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(
                name="delayed-acks", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="dsack", value_type="enum", enum_values=("disabled", "enabled")),
            BigipPropertySpec(
                name="early-retransmit", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="ecn", value_type="enum", enum_values=("disabled", "enabled")),
            BigipPropertySpec(
                name="enhanced-loss-recovery",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="fast-open", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="fast-open-cookie-expiration", value_type="integer"),
            BigipPropertySpec(name="fin-wait-timeout", value_type="integer"),
            BigipPropertySpec(name="fin-wait-2-timeout", value_type="integer"),
            BigipPropertySpec(
                name="hardware-syn-cookie", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="idle-timeout", value_type="integer"),
            BigipPropertySpec(name="init-cwnd", value_type="integer"),
            BigipPropertySpec(name="init-rwnd", value_type="integer"),
            BigipPropertySpec(name="ip-tos-to-client", value_type="integer"),
            BigipPropertySpec(name="keep-alive-interval", value_type="integer"),
            BigipPropertySpec(
                name="limited-transmit", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="link-qos-to-client", value_type="integer"),
            BigipPropertySpec(name="max-retrans", value_type="integer"),
            BigipPropertySpec(name="max-segment-size", value_type="integer"),
            BigipPropertySpec(
                name="md5-signature", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(
                name="md5-signature-passphrase", value_type="boolean", allow_none=True
            ),
            BigipPropertySpec(name="minimum-rto", value_type="integer"),
            BigipPropertySpec(
                name="mptcp", value_type="enum", enum_values=("disabled", "enabled", "passthrough")
            ),
            BigipPropertySpec(
                name="mptcp-csum", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(
                name="mptcp-csum-verify", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(
                name="mptcp-debug", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(
                name="mptcp-fallback",
                value_type="enum",
                enum_values=("reset", "retransmit", "active-accept", "accept"),
            ),
            BigipPropertySpec(name="mptcp-join-max", value_type="integer"),
            BigipPropertySpec(
                name="mptcp-nojoindssack", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="mptcp-rtomax", value_type="integer"),
            BigipPropertySpec(name="mptcp-rxmitmin", value_type="integer"),
            BigipPropertySpec(name="mptcp-subflowmax", value_type="integer"),
            BigipPropertySpec(
                name="mptcp-makeafterbreak", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="mptcp-timeout", value_type="integer"),
            BigipPropertySpec(
                name="mptcp-fastjoin", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(
                name="nagle", value_type="enum", enum_values=("disabled", "enabled", "auto")
            ),
            BigipPropertySpec(name="pkt-loss-ignore-rate", value_type="integer"),
            BigipPropertySpec(name="pkt-loss-ignore-burst", value_type="integer"),
            BigipPropertySpec(name="proxy-buffer-high", value_type="integer"),
            BigipPropertySpec(name="proxy-buffer-low", value_type="integer"),
            BigipPropertySpec(
                name="proxy-mss", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(
                name="proxy-options", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(
                name="push-flag",
                value_type="enum",
                allow_none=True,
                enum_values=("default", "none", "one", "auto"),
            ),
            BigipPropertySpec(
                name="ip-df-mode", value_type="enum", enum_values=("preserve", "set", "clear")
            ),
            BigipPropertySpec(
                name="ip-ttl-mode",
                value_type="enum",
                enum_values=("proxy", "preserve", "decrement", "set"),
            ),
            BigipPropertySpec(name="ip-ttl-value", value_type="integer"),
            BigipPropertySpec(
                name="rate-pace", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="rate-pace-max-rate", value_type="integer"),
            BigipPropertySpec(name="receive-window-size", value_type="integer"),
            BigipPropertySpec(
                name="reset-on-timeout", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="rexmt-thresh", value_type="integer"),
            BigipPropertySpec(
                name="selective-acks", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(
                name="selective-nack", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="send-buffer-size", value_type="integer"),
            BigipPropertySpec(
                name="slow-start", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(
                name="syn-cookie-enable", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(
                name="syn-cookie-whitelist", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="syn-max-retrans", value_type="integer"),
            BigipPropertySpec(name="syn-rto-base", value_type="integer"),
            BigipPropertySpec(
                name="tail-loss-probe", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(
                name="time-wait-recycle", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="time-wait-timeout", value_type="integer"),
            BigipPropertySpec(
                name="timestamps", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(
                name="verified-accept", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="zero-window-timeout", value_type="integer"),
            BigipPropertySpec(name="reset-stats", value_type="string"),
        ),
    )
