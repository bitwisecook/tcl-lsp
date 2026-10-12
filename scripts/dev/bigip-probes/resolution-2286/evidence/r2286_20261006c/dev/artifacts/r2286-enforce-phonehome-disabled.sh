#!/usr/bin/env bash
set -u

state_dir=/config/phonehome-disabled
log_file="$state_dir/disable.log"
stamp=$(date -u +%Y%m%dT%H%M%SZ).$$
mkdir -p "$state_dir"
printf '%s enforcing BIG-IP phone-home disable after boot\n' "$stamp" >> "$log_file"

bigstart stop telemd >> "$log_file" 2>&1 || true
bigstart remove telemd >> "$log_file" 2>&1 || true

cron_file=$(mktemp /var/tmp/phonehome-cron.XXXXXX)
crontab -l 2>/dev/null | grep -v '/usr/bin/phonehome_upload' > "$cron_file" || true
crontab "$cron_file"
rm -f "$cron_file"

mount -o remount,rw /usr
remount_read_only() {
    mount -o remount,ro /usr
}
trap remount_read_only EXIT
for executable in /usr/bin/phonehome_upload /usr/bin/telemd; do
    if [ -e "$executable" ]; then
        digest=$(sha256sum "$executable" | awk '{print $1}')
        disabled="${executable}.disabled.${digest}.${stamp}"
        mv "$executable" "$disabled"
        chmod a-x "$disabled"
        printf '%s moved %s to %s and removed execute permission\n' "$stamp" "$executable" "$disabled" >> "$log_file"
    fi
done
printf '%s BIG-IP phone-home executable paths disabled\n' "$stamp" >> "$log_file"
