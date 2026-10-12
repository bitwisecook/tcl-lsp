set -u
mkdir -p /var/tmp/tcl_lsp_nh20261012exact2/evidence/__tcl_lsp_probe_nh20261012exact2_5_4
E=/var/tmp/tcl_lsp_nh20261012exact2/evidence/__tcl_lsp_probe_nh20261012exact2_5_4
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
run before-0 tmsh list ltm rule /Common/__tcl_lsp_probe_nh20261012exact2_5_4
if ! grep -q "was not found" "$E/before-0.stderr" "$E/before-0.stdout"; then echo "Collision or unverified absence" >&2; exit 3; fi
run input-hash sha256sum /var/tmp/tcl_lsp_nh20261012exact2/__tcl_lsp_probe_nh20261012exact2_5_4.conf
if ! grep -q "^c2d8c3e1c2e6c3d62b285e0d7063345394dce2e2d9d2b14b3d29fa4a33047148 " "$E/input-hash.stdout"; then exit 4; fi
owned+=('ltm rule /Common/__tcl_lsp_probe_nh20261012exact2_5_4')
run load tmsh load sys config merge file /var/tmp/tcl_lsp_nh20261012exact2/__tcl_lsp_probe_nh20261012exact2_5_4.conf
run applied-0 tmsh list ltm rule /Common/__tcl_lsp_probe_nh20261012exact2_5_4
sleep 5
run runtime-logs grep -F 'NHO|nh20261012exact2|' /var/log/ltm
echo "configuration_saved=false" > "$E/save-status.txt"
