set -u
mkdir -p /var/tmp/tcl_lsp_nh20261012app1/evidence/__tcl_lsp_probe_nh20261012app1_0
E=/var/tmp/tcl_lsp_nh20261012app1/evidence/__tcl_lsp_probe_nh20261012app1_0
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
run before-0 tmsh list ltm rule /Common/__tcl_lsp_probe_nh20261012app1_0_provider
if ! grep -q "was not found" "$E/before-0.stderr" "$E/before-0.stdout"; then echo "Collision or unverified absence" >&2; exit 3; fi
run before-1 tmsh list ltm rule /Common/__tcl_lsp_probe_nh20261012app1_0_other
if ! grep -q "was not found" "$E/before-1.stderr" "$E/before-1.stdout"; then echo "Collision or unverified absence" >&2; exit 3; fi
run before-2 tmsh list ltm rule /Common/__tcl_lsp_probe_nh20261012app1_0
if ! grep -q "was not found" "$E/before-2.stderr" "$E/before-2.stdout"; then echo "Collision or unverified absence" >&2; exit 3; fi
run input-hash sha256sum /var/tmp/tcl_lsp_nh20261012app1/__tcl_lsp_probe_nh20261012app1_0.conf
if ! grep -q "^253d46702da9f787ce37c96ac68d5918c6c3ebea3d63680c05ca3f7f49b52fad " "$E/input-hash.stdout"; then exit 4; fi
owned+=('ltm rule /Common/__tcl_lsp_probe_nh20261012app1_0_provider')
owned+=('ltm rule /Common/__tcl_lsp_probe_nh20261012app1_0_other')
owned+=('ltm rule /Common/__tcl_lsp_probe_nh20261012app1_0')
run load tmsh load sys config merge file /var/tmp/tcl_lsp_nh20261012app1/__tcl_lsp_probe_nh20261012app1_0.conf
run applied-0 tmsh list ltm rule /Common/__tcl_lsp_probe_nh20261012app1_0_provider
run applied-1 tmsh list ltm rule /Common/__tcl_lsp_probe_nh20261012app1_0_other
run applied-2 tmsh list ltm rule /Common/__tcl_lsp_probe_nh20261012app1_0
sleep 5
run runtime-logs grep -F 'NHO|nh20261012app1|' /var/log/ltm
echo "configuration_saved=false" > "$E/save-status.txt"
