set -u
mkdir -p /var/tmp/tcl_lsp_nh20261012app3/evidence/__tcl_lsp_probe_nh20261012app3_0
E=/var/tmp/tcl_lsp_nh20261012app3/evidence/__tcl_lsp_probe_nh20261012app3_0
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
run before-0 tmsh list ltm rule /Common/__tcl_lsp_probe_nh20261012app3_0_provider
if ! grep -q "was not found" "$E/before-0.stderr" "$E/before-0.stdout"; then echo "Collision or unverified absence" >&2; exit 3; fi
run before-1 tmsh list ltm rule /Common/__tcl_lsp_probe_nh20261012app3_0_other
if ! grep -q "was not found" "$E/before-1.stderr" "$E/before-1.stdout"; then echo "Collision or unverified absence" >&2; exit 3; fi
run before-2 tmsh list ltm rule /Common/__tcl_lsp_probe_nh20261012app3_0
if ! grep -q "was not found" "$E/before-2.stderr" "$E/before-2.stdout"; then echo "Collision or unverified absence" >&2; exit 3; fi
run input-hash sha256sum /var/tmp/tcl_lsp_nh20261012app3/__tcl_lsp_probe_nh20261012app3_0.conf
if ! grep -q "^b1052a35d05f078311abfa8047e70beee8140fb304cd787cc8bfa5494f1becfe " "$E/input-hash.stdout"; then exit 4; fi
owned+=('ltm rule /Common/__tcl_lsp_probe_nh20261012app3_0_provider')
owned+=('ltm rule /Common/__tcl_lsp_probe_nh20261012app3_0_other')
owned+=('ltm rule /Common/__tcl_lsp_probe_nh20261012app3_0')
run load tmsh load sys config merge file /var/tmp/tcl_lsp_nh20261012app3/__tcl_lsp_probe_nh20261012app3_0.conf
run applied-0 tmsh list ltm rule /Common/__tcl_lsp_probe_nh20261012app3_0_provider
run applied-1 tmsh list ltm rule /Common/__tcl_lsp_probe_nh20261012app3_0_other
run applied-2 tmsh list ltm rule /Common/__tcl_lsp_probe_nh20261012app3_0
sleep 5
run runtime-logs grep -F 'NHO|nh20261012app3|' /var/log/ltm
echo "configuration_saved=false" > "$E/save-status.txt"
