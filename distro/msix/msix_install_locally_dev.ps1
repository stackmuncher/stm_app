# A collection of dev scripts and snippets

<# test-register the package without installing
get-appxpackage |  Where-Object { $_.Name -like "*stackmuncher*" } | Remove-AppxPackage
Add-AppxPackage -Register distro\msix\content\appxmanifest.xml

#>
<# test-install the package
get-appxpackage |  Where-Object { $_.Name -like "*stackmuncher*" } | Remove-AppxPackage
Add-AppxPackage distro\msix\stackmuncher.msix

#>


# test-install the package
# Add-AppxPackage -Register distro\msix\content\appxmanifest.xml
# Get-AppxPackage
# Remove-AppxPackage -package stackmuncher_1.1.1.0_x64__8jrgd7tsqke10
# get-package |  Where-Object { $_.Name -like "*stackmuncher*" }
# get-appxpackage |  Where-Object { $_.Name -like "*stackmuncher*" }
# get-appxpackage |  Where-Object { $_.Name -like "*stackmuncher*" } | Remove-AppxPackage

# installed packages
#get-package
#get-package stackmuncher
#uninstall-package stackmuncher

# Add-AppxPackage "https://distro.stackmuncher.com/msix/stackmuncher.appinstaller" -AppInstallerFile