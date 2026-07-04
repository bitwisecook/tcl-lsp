"""Tests for the SSL certificate inventory / expiry tab.

The lab UCS fixtures terminate no TLS, so an inline config with a
``sys file ssl-cert`` stanza exercises the projection + cross-reference.
"""
from __future__ import annotations

from f5report.report import build_report, collect_model

SCF = """
sys global-settings {
    hostname tls-test.example.net
}
sys file ssl-cert /Common/www.example.com.crt {
    cache-path /config/filestore/files_d/Common_d/certificate_d/:Common:www.example.com.crt_1_1
    expiration-date 1893456000
    expiration-string "Jan  1 00:00:00 2030 GMT"
    fingerprint "SHA256/AA:BB:CC"
    is-bundle false
    issuer "CN=Example Root CA,O=Example,C=US"
    key-size 2048
    key-type rsa-public
    subject "CN=www.example.com,O=Example,C=US"
    subject-alternative-name "DNS:www.example.com, DNS:example.com"
}
ltm profile client-ssl /Common/www_clientssl {
    cert /Common/www.example.com.crt
    key /Common/www.example.com.key
    defaults-from /Common/clientssl
}
ltm virtual /Common/www_https_vs {
    destination /Common/198.51.100.10:443
    ip-protocol tcp
    mask 255.255.255.255
    profiles {
        /Common/www_clientssl { }
        /Common/http { }
    }
    source 0.0.0.0/0
}
"""


def test_cert_model_and_crossref():
    m = collect_model([("inline.scf", SCF)], title="TLS Test")
    d = m["devices"][0]
    certs = d["certificates"]
    assert len(certs) == 1
    c = certs[0]
    assert c["name"] == "www.example.com.crt"
    assert c["fullPath"] == "/Common/www.example.com.crt"
    assert "www.example.com" in c["subject"]
    assert "Example Root CA" in c["issuer"]
    assert c["expirationDate"] == "1893456000"
    assert "2030" in c["expirationString"]
    assert "example.com" in c["subjectAlternativeName"]
    assert "www_clientssl" in c["usedByProfiles"]
    assert "www_https_vs" in c["usedByVirtuals"]
    assert d["counts"]["certificates"] == 1
    assert m["totals"]["certificates"] == 1


def test_report_has_cert_tab():
    html = build_report([("inline.scf", SCF)], title="Cert Report")
    assert 'data-panel="certificates"' in html
    assert "www.example.com.crt" in html
    assert 'data-epoch="1893456000"' in html
    # the live-expiry script is embedded
    assert "cert-remaining" in html


# --- f5mku secret decryption + private-key passphrase ------------------------

# Known key/plaintext pair from tcl-f5mku's test vectors.
F5MKU_KEY = "BHDLd0bbao1VlwpTk1sioQ=="
SCF_ENC = """
sys file ssl-cert /Common/www.example.com.crt {
    expiration-date 1893456000
    expiration-string "Jan  1 00:00:00 2030 GMT"
    subject "CN=www.example.com"
}
sys file ssl-key /Common/www.example.com.key {
    security-type password
    passphrase $M$iP$rr0su9oHn9J9p1t3nRzydA==
}
ltm profile client-ssl /Common/www_clientssl {
    cert /Common/www.example.com.crt
    key /Common/www.example.com.key
}
ltm virtual /Common/www_https_vs {
    destination /Common/198.51.100.10:443
    profiles { /Common/www_clientssl { } }
}
"""


def test_key_passphrase_encrypted_without_master_key():
    c = collect_model([("inline.scf", SCF_ENC)], title="TLS")["devices"][0]["certificates"][0]
    assert c["hasKey"]
    assert c["keyPassphraseEncrypted"]
    assert c["keyPassphrase"].startswith("$M$")


def test_master_key_decrypts_passphrase():
    c = collect_model([("inline.scf", SCF_ENC)], title="TLS", master_key=F5MKU_KEY)["devices"][0]["certificates"][0]
    assert c["keyPassphrase"] == "KEY45678"
    assert not c["keyPassphraseEncrypted"]


def test_wrong_master_key_raises():
    import pytest
    with pytest.raises(Exception):
        collect_model([("inline.scf", SCF_ENC)], title="TLS", master_key="AAAAAAAAAAAAAAAAAAAAAA==")
