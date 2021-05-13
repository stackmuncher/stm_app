# Increments the build number leaving other parts of the version untouched
# Version attribute in the source file must sit on the same line as its element name to minimise 
# the chance of collision with other similar strings in the document
# THERE IS MINIMAL ERROR HANDLING - the script relies on the input data being consistent

# MS AppInstaller has some weird limitations of how many digits can be placed in what part of the version.
# It is not semver compliant. See https://stackoverflow.com/questions/64381453/appinstaller-xml-issue
# Min ver: 0.0.0.0, max 65535.65535.65535.9, so we leave the 4th digit as 0 and put the build in the 3rd (revision)

$ErrorActionPreference = "Stop"

# get an incremented build number
$manifest = Get-Content -Path distro\msix\manifest_x64\appxmanifest.xml -Raw
$matched = $manifest -match ' Version="(\d+\.\d+\.\d+\.0)"'
if (-not $matched) {throw "Failed to match Version string (1)"}
$version_old = $Matches[1]
$matched = $manifest -match ' Version="\d+\.\d+\.(\d+)\.0"'
if (-not $matched) {throw "Failed to match Version string (2)"}
$build_new = ($Matches[1] -as [int]) + 1
$matched = $manifest -match ' Version="(\d+\.\d+\.)\d+\.0"'
if (-not $matched) {throw "Failed to match Version string (3)"}
$version_new = $Matches[1] + $build_new + ".0"
write-host "Version inclrement: $version_old --> $version_new"

# set the version in appxmanifest
$matched = $manifest -match ' Version="(\d+\.\d+\.\d+\.0)"'
if (-not $matched) {throw "Failed to match Version string (4)"}
$manifest -replace $Matches[1], $version_new | Set-Content -Path distro\msix\manifest_x64\appxmanifest.xml -Force

# set the version in appinstaller
$manifest = Get-Content -Path distro\msix\stackmuncher_x64.appinstaller -Raw
$matched = $manifest -match '<MainPackage.+ Version="(\d+\.\d+\.\d+\.0)"'
if (-not $matched) {throw "Failed to match Version string (5)"}
$manifest -replace $Matches[1], $version_new | Set-Content -Path distro\msix\stackmuncher_x64.appinstaller -Force
