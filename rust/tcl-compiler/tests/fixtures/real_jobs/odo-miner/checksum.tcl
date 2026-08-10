# Copyright (C) 2019 MentalCollatz
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU General Public License for more details.
#
# You should have received a copy of the GNU General Public License
# along with this program.  If not, see <http://www.gnu.org/licenses/>.

proc crc_shift {crcval} {
    return [expr {($crcval >> 1) ^ (-($crcval & 1) & 0x9fc)}]
}

# Pre-compute checksums for contiguous bit flips, which seems to be the most
# common failure type.
array set crc_correction_tab {}
proc crc_setup {} {
    global crc_correction_tab

    set mask 0x800
    set masks [list]
    for {set i 43} {$i >= 0} {incr i -1} {
        lappend masks $mask
        set mask [crc_shift $mask]
    }
    set masks [lreverse $masks]

    for {set i 0} {$i < 44} {incr i} {
        set correction 0
        set mask 0
        for {set j $i} {$j < 44} {incr j} {
            set mask [expr {$mask ^ [lindex $masks $j]}]
            set correction [expr {$correction | (1 << $j)}]
            array set crc_correction_tab [list $mask $correction]
        }
    }
}

proc crc_message {padded cksum recoverable} {
    if {$recoverable} {
        status_print -type warning "Recoverable checkcksum failure: $padded -> $cksum"
    } else {
        status_print -type error "Unrecoverable checkcksum failure: $padded -> $cksum"
    }
}

proc crc_check {padded} {
    global crc_correction_tab

    set cksum $padded
    for {set i 0} {$i < 32} {incr i} {
        set cksum [crc_shift $cksum]
    }
    # If everything went okay, cksum should be 0
    if {$cksum != 0} {
        # Try to recover
        set item [array get crc_correction_tab $cksum]
        set recoverable [expr {$item != ""}]
        crc_message $padded $cksum $recoverable
        if {$recoverable} {
            set padded [expr {$padded ^ [lindex $item 1]}]
        }
    }
    return [expr {$padded & 0xffffffff}]
}

crc_setup
