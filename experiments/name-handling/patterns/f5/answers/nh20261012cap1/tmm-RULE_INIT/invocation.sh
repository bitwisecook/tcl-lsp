set -u
mkdir -p /var/tmp/tcl_lsp_nh20261012cap1/evidence/__tcl_lsp_probe_nh20261012cap1_tmm_RULE_INIT
E=/var/tmp/tcl_lsp_nh20261012cap1/evidence/__tcl_lsp_probe_nh20261012cap1_tmm_RULE_INIT
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
run before-0 tmsh list ltm rule /Common/__tcl_lsp_probe_nh20261012cap1_tmm_RULE_INIT
if ! grep -q "was not found" "$E/before-0.stderr" "$E/before-0.stdout"; then echo "Collision or unverified absence" >&2; exit 3; fi
run input-hash sha256sum /var/tmp/tcl_lsp_nh20261012cap1/__tcl_lsp_probe_nh20261012cap1_tmm_RULE_INIT.conf
if ! grep -q "^2e56e18bd80a394cb1835b346587b51035736f2c1703fefb6a5a3ce5cf5ef7b1 " "$E/input-hash.stdout"; then exit 4; fi
owned+=('ltm rule /Common/__tcl_lsp_probe_nh20261012cap1_tmm_RULE_INIT')
run load tmsh load sys config merge file /var/tmp/tcl_lsp_nh20261012cap1/__tcl_lsp_probe_nh20261012cap1_tmm_RULE_INIT.conf
run applied-0 tmsh list ltm rule /Common/__tcl_lsp_probe_nh20261012cap1_tmm_RULE_INIT
sleep 2
run runtime-logs grep -F 'NHO|nh20261012cap1|' /var/log/ltm
echo "configuration_saved=false" > "$E/save-status.txt"
