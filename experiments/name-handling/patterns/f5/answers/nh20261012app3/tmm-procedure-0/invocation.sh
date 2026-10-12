set -u
mkdir -p /var/tmp/tcl_lsp_nh20261012app3/evidence/__tcl_lsp_probe_nh20261012app3_1
E=/var/tmp/tcl_lsp_nh20261012app3/evidence/__tcl_lsp_probe_nh20261012app3_1
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
run before-0 tmsh list ltm rule /Common/__tcl_lsp_probe_nh20261012app3_1_provider
if ! grep -q "was not found" "$E/before-0.stderr" "$E/before-0.stdout"; then echo "Collision or unverified absence" >&2; exit 3; fi
run before-1 tmsh list ltm rule /Common/__tcl_lsp_probe_nh20261012app3_1_other
if ! grep -q "was not found" "$E/before-1.stderr" "$E/before-1.stdout"; then echo "Collision or unverified absence" >&2; exit 3; fi
run before-2 tmsh list ltm rule /Common/__tcl_lsp_probe_nh20261012app3_1
if ! grep -q "was not found" "$E/before-2.stderr" "$E/before-2.stdout"; then echo "Collision or unverified absence" >&2; exit 3; fi
run input-hash sha256sum /var/tmp/tcl_lsp_nh20261012app3/__tcl_lsp_probe_nh20261012app3_1.conf
if ! grep -q "^5c7f3d621eb92eb5ff318ca2c6cd0fb2356dce4336bc7b18a4f3ab6fa17e534f " "$E/input-hash.stdout"; then exit 4; fi
owned+=('ltm rule /Common/__tcl_lsp_probe_nh20261012app3_1_provider')
owned+=('ltm rule /Common/__tcl_lsp_probe_nh20261012app3_1_other')
owned+=('ltm rule /Common/__tcl_lsp_probe_nh20261012app3_1')
run load tmsh load sys config merge file /var/tmp/tcl_lsp_nh20261012app3/__tcl_lsp_probe_nh20261012app3_1.conf
run applied-0 tmsh list ltm rule /Common/__tcl_lsp_probe_nh20261012app3_1_provider
run applied-1 tmsh list ltm rule /Common/__tcl_lsp_probe_nh20261012app3_1_other
run applied-2 tmsh list ltm rule /Common/__tcl_lsp_probe_nh20261012app3_1
sleep 5
run runtime-logs grep -F 'NHO|nh20261012app3|' /var/log/ltm
echo "configuration_saved=false" > "$E/save-status.txt"
