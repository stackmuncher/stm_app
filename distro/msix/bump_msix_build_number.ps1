# Increments the build number leaving other parts of the version untouched
# Version attribute in the source file must sit on the same line as its element name to minimise 
# the chance of collision with other similar strings in the document
# THERE IS NO ERROR HANDLING - the script relies on the inout data being consistent

# get an incremented build number
$manifest = Get-Content -Path distro\msix\manifest_x64\appxmanifest.xml -Raw
$matched = $manifest -match ' Version="(\d+\.\d+\.\d+\.\d+)"'
$version_old = $Matches[1]
$matched = $manifest -match ' Version="\d+\.\d+\.\d+\.(\d+)"'
$build_new = ($Matches[1] -as [int]) + 1
$matched = $manifest -match ' Version="(\d+\.\d+\.\d+\.)\d+"'
$version_new = $Matches[1] + $build_new
write-host "Version inclrement: $version_old --> $version_new"

# set the version in appxmanifest
$matched = $manifest -match ' Version="(\d+\.\d+\.\d+\.\d+)"'
$manifest -replace $Matches[1], $version_new | Set-Content -Path distro\msix\manifest_x64\appxmanifest.xml -Force

# set the version in appinstaller
$manifest = Get-Content -Path distro\msix\stackmuncher_x64.appinstaller -Raw
$matched = $manifest -match '<MainPackage.+ Version="(\d+\.\d+\.\d+\.\d+)"'
$manifest -replace $Matches[1], $version_new | Set-Content -Path distro\msix\stackmuncher_x64.appinstaller -Force
