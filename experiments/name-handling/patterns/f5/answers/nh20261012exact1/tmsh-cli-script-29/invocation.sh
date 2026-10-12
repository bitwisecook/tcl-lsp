set -u
mkdir -p /var/tmp/tcl_lsp_nh20261012exact1/evidence/__tcl_lsp_probe_nh20261012exact1_2_29
E=/var/tmp/tcl_lsp_nh20261012exact1/evidence/__tcl_lsp_probe_nh20261012exact1_2_29
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
run before-0 tmsh list cli script /Common/__tcl_lsp_probe_nh20261012exact1_2_29
if ! grep -q "was not found" "$E/before-0.stderr" "$E/before-0.stdout"; then echo "Collision or unverified absence" >&2; exit 3; fi
run input-hash sha256sum /var/tmp/tcl_lsp_nh20261012exact1/__tcl_lsp_probe_nh20261012exact1_2_29.conf
if ! grep -q "^0b614d652be717a925c493578b8ef8a695eac85087f89093149acb6866f3dbd0 " "$E/input-hash.stdout"; then exit 4; fi
owned+=('cli script /Common/__tcl_lsp_probe_nh20261012exact1_2_29')
run load tmsh load sys config merge file /var/tmp/tcl_lsp_nh20261012exact1/__tcl_lsp_probe_nh20261012exact1_2_29.conf
run applied-0 tmsh list cli script /Common/__tcl_lsp_probe_nh20261012exact1_2_29
run invoke tmsh run cli script /Common/__tcl_lsp_probe_nh20261012exact1_2_29
sleep 5
run runtime-logs grep -F 'NHO|nh20261012exact1|' /var/log/ltm
echo "configuration_saved=false" > "$E/save-status.txt"
