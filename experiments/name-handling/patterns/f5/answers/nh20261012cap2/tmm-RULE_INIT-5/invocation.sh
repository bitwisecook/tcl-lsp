set -u
mkdir -p /var/tmp/tcl_lsp_nh20261012cap2/evidence/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_5
E=/var/tmp/tcl_lsp_nh20261012cap2/evidence/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_5
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
run before-0 tmsh list ltm rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_5
if ! grep -q "was not found" "$E/before-0.stderr" "$E/before-0.stdout"; then echo "Collision or unverified absence" >&2; exit 3; fi
run input-hash sha256sum /var/tmp/tcl_lsp_nh20261012cap2/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_5.conf
if ! grep -q "^e38c6288bd0e437728a914b6730a01d5b5101485ef91f159dce13532e59a1b8d " "$E/input-hash.stdout"; then exit 4; fi
owned+=('ltm rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_5')
run load tmsh load sys config merge file /var/tmp/tcl_lsp_nh20261012cap2/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_5.conf
run applied-0 tmsh list ltm rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_5
sleep 2
run runtime-logs grep -F 'NHO|nh20261012cap2|' /var/log/ltm
echo "configuration_saved=false" > "$E/save-status.txt"
