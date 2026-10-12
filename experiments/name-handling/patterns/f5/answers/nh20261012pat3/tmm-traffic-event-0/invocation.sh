# Sanitised retained source: transport/device literals changed; these are not original captured bytes.
set -u
mkdir -p /var/tmp/tcl_lsp_nh20261012pat3/evidence/__tcl_lsp_probe_nh20261012pat3_7_0
E=/var/tmp/tcl_lsp_nh20261012pat3/evidence/__tcl_lsp_probe_nh20261012pat3_7_0
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
run before-0 tmsh list ltm rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0
if ! grep -q "was not found" "$E/before-0.stderr" "$E/before-0.stdout"; then echo "Collision or unverified absence" >&2; exit 3; fi
run before-1 tmsh list ltm virtual /Common/__tcl_lsp_probe_nh20261012pat3_7_0_vs
if ! grep -q "was not found" "$E/before-1.stderr" "$E/before-1.stdout"; then echo "Collision or unverified absence" >&2; exit 3; fi
run destination-inventory tmsh list ltm virtual one-line
if grep -q "destination 192.0.2.10:19912 " "$E/destination-inventory.stdout"; then echo "Destination collision" >&2; exit 3; fi
run input-hash sha256sum /var/tmp/tcl_lsp_nh20261012pat3/__tcl_lsp_probe_nh20261012pat3_7_0.conf
if ! grep -q "^13cc2657b19c4f6ae078456fa4171ca6bb21fa7f674bd0e625a618457bfcc413 " "$E/input-hash.stdout"; then exit 4; fi
owned+=('ltm rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0')
owned+=('ltm virtual /Common/__tcl_lsp_probe_nh20261012pat3_7_0_vs')
run load tmsh load sys config merge file /var/tmp/tcl_lsp_nh20261012pat3/__tcl_lsp_probe_nh20261012pat3_7_0.conf
run applied-0 tmsh list ltm rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0
run applied-1 tmsh list ltm virtual /Common/__tcl_lsp_probe_nh20261012pat3_7_0_vs
if [ "$(cat "$E/load.exit")" = 0 ] && [ "$(cat "$E/applied-1.exit")" = 0 ]; then touch "$E/traffic-ready"; for n in {1..90}; do [ -e "$E/traffic-done" ] && break; sleep 1; done; fi
sleep 5
run runtime-logs grep -F 'NHO|nh20261012pat3|' /var/log/ltm
echo "configuration_saved=false" > "$E/save-status.txt"
