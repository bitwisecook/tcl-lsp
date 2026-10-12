set -u
mkdir -p /var/tmp/tcl_lsp_nh20261012literal1/evidence/__tcl_lsp_probe_nh20261012literal1_3_4
E=/var/tmp/tcl_lsp_nh20261012literal1/evidence/__tcl_lsp_probe_nh20261012literal1_3_4
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
run before-0 tmsh list ltm rule /Common/__tcl_lsp_probe_nh20261012literal1_3_4
if ! grep -q "was not found" "$E/before-0.stderr" "$E/before-0.stdout"; then echo "Collision or unverified absence" >&2; exit 3; fi
run input-hash sha256sum /var/tmp/tcl_lsp_nh20261012literal1/__tcl_lsp_probe_nh20261012literal1_3_4.conf
if ! grep -q "^9684ecc17aa2e66cf75653ea22d63cfd5916ad2905e863e952ae650ef30fe921 " "$E/input-hash.stdout"; then exit 4; fi
owned+=('ltm rule /Common/__tcl_lsp_probe_nh20261012literal1_3_4')
run load tmsh load sys config merge file /var/tmp/tcl_lsp_nh20261012literal1/__tcl_lsp_probe_nh20261012literal1_3_4.conf
run applied-0 tmsh list ltm rule /Common/__tcl_lsp_probe_nh20261012literal1_3_4
sleep 5
run runtime-logs grep -F 'NHO|nh20261012literal1|' /var/log/ltm
echo "configuration_saved=false" > "$E/save-status.txt"
