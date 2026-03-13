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
            "sys_log_config_filter",
            module="sys",
            object_types=("log-config filter",),
        ),
        header_types=(("sys", "log-config filter"),),
        properties=(
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="level",
                value_type="enum",
                enum_values=("alert", "crit", "debug", "emerg", "err", "info", "notice", "warn"),
            ),
            BigipPropertySpec(name="message-id", value_type="integer", allow_none=True),
            BigipPropertySpec(name="publisher", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="source", value_type="string"),
            BigipPropertySpec(name="api-protection", value_type="string"),
            BigipPropertySpec(name="based", value_type="string"),
            BigipPropertySpec(name="bigstart", value_type="string"),
            BigipPropertySpec(name="common-f5logging", value_type="string"),
            BigipPropertySpec(name="daemon", value_type="string"),
            BigipPropertySpec(name="eca", value_type="string"),
            BigipPropertySpec(name="em-file", value_type="string"),
            BigipPropertySpec(name="fflag", value_type="string"),
            BigipPropertySpec(name="guestagentd", value_type="string"),
            BigipPropertySpec(name="hornet-nest-flow-manager", value_type="string"),
            BigipPropertySpec(name="hornet-text-client", value_type="string"),
            BigipPropertySpec(name="icrd", value_type="reference", references=("ltm_rule",)),
            BigipPropertySpec(name="ivs", value_type="string"),
            BigipPropertySpec(name="map", value_type="string"),
            BigipPropertySpec(name="mcpd-dev", value_type="string"),
            BigipPropertySpec(name="mcpd-net", value_type="string"),
            BigipPropertySpec(name="mdm", value_type="string"),
            BigipPropertySpec(name="network", value_type="boolean"),
            BigipPropertySpec(name="pkcs11d", value_type="string"),
            BigipPropertySpec(name="promptstatusd", value_type="string"),
            BigipPropertySpec(name="rewrite", value_type="reference", references=("ltm_rule",)),
            BigipPropertySpec(name="saas", value_type="string"),
            BigipPropertySpec(name="spolicy", value_type="string"),
            BigipPropertySpec(name="ssl-handshake", value_type="string"),
            BigipPropertySpec(name="stpd", value_type="string"),
            BigipPropertySpec(name="tmm-tcp", value_type="string"),
            BigipPropertySpec(name="webssh", value_type="string"),
            BigipPropertySpec(name="icr-eventd", value_type="string"),
        ),
    )
