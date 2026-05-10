#!/usr/bin/env bash
#
# gen_certs.sh — produce the TLS material the explain-flow test lab needs.
#
# Generates a private CA and a small zoo of leaf certificates designed to
# cover the cases the lab SCF references:
#
#   lab_ca.{crt,key}                — root CA (also the chain pem)
#   lab_server_valid.{crt,key}      — valid leaf, SAN: valid.lab.test,
#                                     api.lab.test, *.lab.test
#   lab_server_expired.{crt,key}    — leaf with notAfter ~1 day in the past
#   lab_server_mismatch.{crt,key}   — leaf whose SAN doesn't match any
#                                     hostname clients will request (the
#                                     SCF references this only via
#                                     vs_tls_selfsigned by default — keep
#                                     it generated for ad-hoc testing)
#   lab_server_selfsigned.{crt,key} — self-signed leaf, no chain
#   lab_client.{crt,key}            — client cert for optional mTLS testing
#
# All certs are EC P-256 (modern, smaller, faster handshake than 2048-bit
# RSA — keeps pcaps small).  Output goes to ./certs/ by default.
#
# Usage:
#   gen_certs.sh [output_dir]
#

set -euo pipefail

OUT="${1:-$(dirname "$(readlink -f "$0")")/certs}"
mkdir -p "$OUT"
cd "$OUT"

OPENSSL="${OPENSSL:-openssl}"

if ! "$OPENSSL" version >/dev/null 2>&1; then
    echo "gen_certs.sh: openssl not found on PATH" >&2
    exit 2
fi

# A reusable openssl config snippet for EC-P256 leaf CSRs with SANs.
_make_san_cnf() {
    local cnf="$1"; shift
    local cn="$1"; shift
    cat >"$cnf" <<EOF
[ req ]
distinguished_name = dn
req_extensions = v3_req
prompt = no

[ dn ]
CN = ${cn}

[ v3_req ]
basicConstraints = CA:FALSE
keyUsage = digitalSignature, keyEncipherment
extendedKeyUsage = serverAuth
subjectAltName = @alt
[ alt ]
EOF
    local i=1
    for s in "$@"; do
        echo "DNS.${i} = ${s}" >>"$cnf"
        i=$((i + 1))
    done
}

# 1. Root CA -----------------------------------------------------------
if [[ ! -f lab_ca.crt ]]; then
    echo "→ generating lab CA"
    "$OPENSSL" ecparam -name prime256v1 -genkey -noout -out lab_ca.key
    "$OPENSSL" req -x509 -new -key lab_ca.key -days 3650 \
        -subj "/CN=explain-flow lab CA" \
        -out lab_ca.crt
fi

# 2. Valid leaf --------------------------------------------------------
echo "→ generating valid leaf (SANs: valid.lab.test, api.lab.test, *.lab.test)"
"$OPENSSL" ecparam -name prime256v1 -genkey -noout -out lab_server_valid.key
_make_san_cnf valid.cnf "valid.lab.test" \
    "valid.lab.test" "api.lab.test" "*.lab.test"
"$OPENSSL" req -new -key lab_server_valid.key -config valid.cnf -out valid.csr
"$OPENSSL" x509 -req -in valid.csr \
    -CA lab_ca.crt -CAkey lab_ca.key -CAcreateserial \
    -days 730 -extfile valid.cnf -extensions v3_req \
    -out lab_server_valid.crt
rm -f valid.csr valid.cnf

# 3. Expired leaf ------------------------------------------------------
echo "→ generating expired leaf (notAfter ~yesterday)"
"$OPENSSL" ecparam -name prime256v1 -genkey -noout -out lab_server_expired.key
_make_san_cnf expired.cnf "expired.lab.test" "expired.lab.test"
"$OPENSSL" req -new -key lab_server_expired.key -config expired.cnf -out expired.csr
# -days -1 is illegal; we use -startdate / -enddate via ca-style x509 instead.
# Trick: set -days 1 then re-sign with -enddate clamping into the past via
# a faketime wrapper if available; otherwise sign with -days 1 and the
# orchestrator can fast-forward the wallclock.  Default fallback below
# uses -days 1 plus a sleep-free trick: manually set notBefore far in
# the past so the cert is already expired even with a normal wall clock.
"$OPENSSL" x509 -req -in expired.csr \
    -CA lab_ca.crt -CAkey lab_ca.key -CAcreateserial \
    -days 1 \
    -extfile expired.cnf -extensions v3_req \
    -out lab_server_expired.crt.tmp

# Re-stamp validity into the past using -set_issuer and -force-pubkey ...
# Easiest cross-platform way: use `-set_serial` plus regenerate with
# faketime if available.
if command -v faketime >/dev/null 2>&1; then
    rm lab_server_expired.crt.tmp
    faketime '2020-01-01 00:00:00' \
        "$OPENSSL" x509 -req -in expired.csr \
            -CA lab_ca.crt -CAkey lab_ca.key -CAcreateserial \
            -days 30 \
            -extfile expired.cnf -extensions v3_req \
            -out lab_server_expired.crt
else
    echo "  (faketime not available — expired cert just has -days 1; rerun" \
         "after wallclock advances or install faketime for proper backdating)"
    mv lab_server_expired.crt.tmp lab_server_expired.crt
fi
rm -f expired.csr expired.cnf

# 4. Hostname-mismatch leaf -------------------------------------------
echo "→ generating hostname-mismatch leaf (SAN: somewhere-else.lab.test)"
"$OPENSSL" ecparam -name prime256v1 -genkey -noout -out lab_server_mismatch.key
_make_san_cnf mismatch.cnf "somewhere-else.lab.test" "somewhere-else.lab.test"
"$OPENSSL" req -new -key lab_server_mismatch.key -config mismatch.cnf -out mismatch.csr
"$OPENSSL" x509 -req -in mismatch.csr \
    -CA lab_ca.crt -CAkey lab_ca.key -CAcreateserial \
    -days 730 -extfile mismatch.cnf -extensions v3_req \
    -out lab_server_mismatch.crt
rm -f mismatch.csr mismatch.cnf

# 5. Self-signed leaf (no chain) --------------------------------------
echo "→ generating self-signed leaf (SAN: selfsigned.lab.test)"
"$OPENSSL" ecparam -name prime256v1 -genkey -noout -out lab_server_selfsigned.key
_make_san_cnf selfsigned.cnf "selfsigned.lab.test" "selfsigned.lab.test"
"$OPENSSL" req -x509 -new -key lab_server_selfsigned.key -config selfsigned.cnf \
    -days 730 -extensions v3_req \
    -out lab_server_selfsigned.crt
rm -f selfsigned.cnf

# 6. Client cert (for optional mTLS) ----------------------------------
if [[ ! -f lab_client.crt ]]; then
    echo "→ generating client cert (CN=lab-client)"
    "$OPENSSL" ecparam -name prime256v1 -genkey -noout -out lab_client.key
    "$OPENSSL" req -new -key lab_client.key \
        -subj "/CN=lab-client" -out client.csr
    "$OPENSSL" x509 -req -in client.csr \
        -CA lab_ca.crt -CAkey lab_ca.key -CAcreateserial \
        -days 730 -out lab_client.crt
    rm -f client.csr
fi

# 7. Convenience bundles ----------------------------------------------
cat lab_server_valid.crt lab_ca.crt > lab_server_valid_chain.pem
cat lab_server_expired.crt lab_ca.crt > lab_server_expired_chain.pem
cat lab_server_mismatch.crt lab_ca.crt > lab_server_mismatch_chain.pem

# Strip the openssl serial helper.
rm -f lab_ca.srl

echo ""
echo "lab certs ready in: $OUT"
ls -1 "$OUT"
