set -u
mkdir -p /var/tmp/tcl_lsp_nh20261012app3/evidence/__tcl_lsp_probe_nh20261012app3_2_cleanup
E=/var/tmp/tcl_lsp_nh20261012app3/evidence/__tcl_lsp_probe_nh20261012app3_2_cleanup
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
run before-0 tmsh list ltm rule /Common/__tcl_lsp_probe_nh20261012app3_2_cleanup
if ! grep -q "was not found" "$E/before-0.stderr" "$E/before-0.stdout"; then echo "Collision or unverified absence" >&2; exit 3; fi
run input-hash sha256sum /var/tmp/tcl_lsp_nh20261012app3/__tcl_lsp_probe_nh20261012app3_2_cleanup.conf
if ! grep -q "^183085f3a0f2f3b7ef4bbf7ee0b32a32b14fd333461796a8cd18a474fa8b156c " "$E/input-hash.stdout"; then exit 4; fi
owned+=('ltm rule /Common/__tcl_lsp_probe_nh20261012app3_2_cleanup')
run load tmsh load sys config merge file /var/tmp/tcl_lsp_nh20261012app3/__tcl_lsp_probe_nh20261012app3_2_cleanup.conf
run applied-0 tmsh list ltm rule /Common/__tcl_lsp_probe_nh20261012app3_2_cleanup
sleep 5
run runtime-logs grep -F 'NHO|nh20261012app3|' /var/log/ltm
echo "configuration_saved=false" > "$E/save-status.txt"
