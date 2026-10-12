set -u
mkdir -p /var/tmp/tcl_lsp_nh20261012exact1/evidence/__tcl_lsp_probe_nh20261012exact1_2_43
E=/var/tmp/tcl_lsp_nh20261012exact1/evidence/__tcl_lsp_probe_nh20261012exact1_2_43
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
run before-0 tmsh list cli script /Common/__tcl_lsp_probe_nh20261012exact1_2_43
if ! grep -q "was not found" "$E/before-0.stderr" "$E/before-0.stdout"; then echo "Collision or unverified absence" >&2; exit 3; fi
run input-hash sha256sum /var/tmp/tcl_lsp_nh20261012exact1/__tcl_lsp_probe_nh20261012exact1_2_43.conf
if ! grep -q "^312011f67ef5966d750f265f02322e6b3fa2f66eb94cd60f7e5e5db241162b61 " "$E/input-hash.stdout"; then exit 4; fi
owned+=('cli script /Common/__tcl_lsp_probe_nh20261012exact1_2_43')
run load tmsh load sys config merge file /var/tmp/tcl_lsp_nh20261012exact1/__tcl_lsp_probe_nh20261012exact1_2_43.conf
run applied-0 tmsh list cli script /Common/__tcl_lsp_probe_nh20261012exact1_2_43
run invoke tmsh run cli script /Common/__tcl_lsp_probe_nh20261012exact1_2_43
sleep 5
run runtime-logs grep -F 'NHO|nh20261012exact1|' /var/log/ltm
echo "configuration_saved=false" > "$E/save-status.txt"
