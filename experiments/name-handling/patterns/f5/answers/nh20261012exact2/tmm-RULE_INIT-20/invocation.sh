set -u
mkdir -p /var/tmp/tcl_lsp_nh20261012exact2/evidence/__tcl_lsp_probe_nh20261012exact2_5_20
E=/var/tmp/tcl_lsp_nh20261012exact2/evidence/__tcl_lsp_probe_nh20261012exact2_5_20
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
run before-0 tmsh list ltm rule /Common/__tcl_lsp_probe_nh20261012exact2_5_20
if ! grep -q "was not found" "$E/before-0.stderr" "$E/before-0.stdout"; then echo "Collision or unverified absence" >&2; exit 3; fi
run input-hash sha256sum /var/tmp/tcl_lsp_nh20261012exact2/__tcl_lsp_probe_nh20261012exact2_5_20.conf
if ! grep -q "^66dc1b4027a7e195249f5bce5137434d26f097706f66ce0d868362241607eaf5 " "$E/input-hash.stdout"; then exit 4; fi
owned+=('ltm rule /Common/__tcl_lsp_probe_nh20261012exact2_5_20')
run load tmsh load sys config merge file /var/tmp/tcl_lsp_nh20261012exact2/__tcl_lsp_probe_nh20261012exact2_5_20.conf
run applied-0 tmsh list ltm rule /Common/__tcl_lsp_probe_nh20261012exact2_5_20
sleep 5
run runtime-logs grep -F 'NHO|nh20261012exact2|' /var/log/ltm
echo "configuration_saved=false" > "$E/save-status.txt"
