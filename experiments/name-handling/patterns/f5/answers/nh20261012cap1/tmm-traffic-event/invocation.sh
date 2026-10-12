set -u
mkdir -p /var/tmp/tcl_lsp_nh20261012cap1/evidence/__tcl_lsp_probe_nh20261012cap1_tmm_traffic_event
E=/var/tmp/tcl_lsp_nh20261012cap1/evidence/__tcl_lsp_probe_nh20261012cap1_tmm_traffic_event
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
run before-0 tmsh list ltm rule /Common/__tcl_lsp_probe_nh20261012cap1_tmm_traffic_event
if ! grep -q "was not found" "$E/before-0.stderr" "$E/before-0.stdout"; then echo "Collision or unverified absence" >&2; exit 3; fi
run before-1 tmsh list ltm virtual /Common/__tcl_lsp_probe_nh20261012cap1_tmm_traffic_event_vs
if ! grep -q "was not found" "$E/before-1.stderr" "$E/before-1.stdout"; then echo "Collision or unverified absence" >&2; exit 3; fi
run input-hash sha256sum /var/tmp/tcl_lsp_nh20261012cap1/__tcl_lsp_probe_nh20261012cap1_tmm_traffic_event.conf
if ! grep -q "^fab1e9b2004591bb53368f1d1138072eba29a95a570056da3c9ff1722fdc6d33 " "$E/input-hash.stdout"; then exit 4; fi
owned+=('ltm rule /Common/__tcl_lsp_probe_nh20261012cap1_tmm_traffic_event')
owned+=('ltm virtual /Common/__tcl_lsp_probe_nh20261012cap1_tmm_traffic_event_vs')
run load tmsh load sys config merge file /var/tmp/tcl_lsp_nh20261012cap1/__tcl_lsp_probe_nh20261012cap1_tmm_traffic_event.conf
run applied-0 tmsh list ltm rule /Common/__tcl_lsp_probe_nh20261012cap1_tmm_traffic_event
run applied-1 tmsh list ltm virtual /Common/__tcl_lsp_probe_nh20261012cap1_tmm_traffic_event_vs
touch "$E/traffic-ready"
sleep 50
sleep 2
run runtime-logs grep -F 'NHO|nh20261012cap1|' /var/log/ltm
echo "configuration_saved=false" > "$E/save-status.txt"
