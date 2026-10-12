set -u
mkdir -p /var/tmp/tcl_lsp_nh20261012cap2/evidence/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_1
E=/var/tmp/tcl_lsp_nh20261012cap2/evidence/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_1
owned=()
run() { label=$1; shift; "$@" > "$E/$label.stdout" 2> "$E/$label.stderr"; echo $? > "$E/$label.exit"; }
cleanup() {
  for (( i=${#owned[@]}-1; i>=0; i-- )); do
    read -r -a words <<< "${owned[$i]}"
    run "delete-$i" tmsh delete "${words[@]}"
    run "after-$i" tmsh list "${words[@]}"
  done
}
trap cleanup EXIT
run before-0 tmsh list ltm rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_1
if ! grep -q "was not found" "$E/before-0.stderr" "$E/before-0.stdout"; then echo "Collision or unverified absence" >&2; exit 3; fi
run input-hash sha256sum /var/tmp/tcl_lsp_nh20261012cap2/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_1.conf
if ! grep -q "^a4cffa21516dc7889979c364dbb9df12e98f31a151fc22a939774c18c2c1bcc5 " "$E/input-hash.stdout"; then exit 4; fi
owned+=('ltm rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_1')
run load tmsh load sys config merge file /var/tmp/tcl_lsp_nh20261012cap2/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_1.conf
run applied-0 tmsh list ltm rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_1
sleep 2
run runtime-logs grep -F 'NHO|nh20261012cap2|' /var/log/ltm
echo "configuration_saved=false" > "$E/save-status.txt"
