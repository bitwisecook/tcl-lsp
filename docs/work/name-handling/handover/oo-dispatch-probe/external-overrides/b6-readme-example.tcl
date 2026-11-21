package require overrides
set overridesDict {
    source {
        lambda {
            {filename} {
                puts "Sourcing $fileName..."
                nextOverride $fileName
            }
        }
    }
}
namespace import overrides::withCmdOverrides overrides::nextOverride
puts [catch {withCmdOverrides $overridesDict { source /dev/null }} m]:$m
