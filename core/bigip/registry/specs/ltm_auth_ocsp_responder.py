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
            "ltm_auth_ocsp_responder",
            module="ltm",
            object_types=("auth ocsp-responder",),
        ),
        header_types=(("ltm", "auth ocsp-responder"),),
        properties=(
            BigipPropertySpec(
                name="allow-certs", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="ca-file", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="ca-path", value_type="boolean", allow_none=True),
            BigipPropertySpec(
                name="cert-id-digest", value_type="enum", enum_values=("md5", "sha1")
            ),
            BigipPropertySpec(name="chain", value_type="enum", enum_values=("disabled", "enabled")),
            BigipPropertySpec(
                name="check-certs", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="explicit", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(
                name="ignore-aia", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(
                name="intern", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="nonce", value_type="enum", enum_values=("disabled", "enabled")),
            BigipPropertySpec(name="sign-digest", value_type="enum", enum_values=("md5", "sha1")),
            BigipPropertySpec(name="sign-key", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="sign-key-pass-phrase", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="sign-other", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="signer", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="status-age", value_type="integer"),
            BigipPropertySpec(
                name="trust-other", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(
                name="url", value_type="enum", allow_none=True, enum_values=("none", "url")
            ),
            BigipPropertySpec(name="va-file", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="validity-period", value_type="integer"),
            BigipPropertySpec(
                name="verify", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(
                name="verify-cert", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="verify-other", value_type="boolean", allow_none=True),
            BigipPropertySpec(
                name="verify-sig", value_type="enum", enum_values=("disabled", "enabled")
            ),
        ),
    )
