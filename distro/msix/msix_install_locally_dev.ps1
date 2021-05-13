# A collection of dev scripts and snippets

<# test-register the package without installing
get-appxpackage |  Where-Object { $_.Name -like "*stackmuncher*" } | Remove-AppxPackage
Add-AppxPackage -Register distro\msix\content\appxmanifest.xml
#>

# install from different sources
get-appxpackage |  Where-Object { $_.Name -like "*stackmuncher*" } | Remove-AppxPackage
Add-AppxPackage "distro\msix\stackmuncher_x64.appinstaller" -AppInstallerFile
#Add-AppxPackage "distro\msix\stackmuncher_x64.msix"
#Add-AppxPackage "https://distro.stackmuncher.com/msix/stackmuncher_x64.msix"
#Add-AppxPackage "https://distro.stackmuncher.com/msix/stackmuncher_x64.appinstaller" -AppInstallerFile
get-appxpackage |  Where-Object { $_.Name -like "*stackmuncher*" }
