# For Windoes Server 2019 and machines where the install should be done via PowerShell

# remove any prior packages
get-appxpackage |  Where-Object { $_.Name -like "*stackmuncher*" } | Remove-AppxPackage

# install directly from S3 with auto-update
Add-AppxPackage "https://distro.stackmuncher.com/msix/stackmuncher_x64.appinstaller" -AppInstallerFile

# install directly from S3 without auto-update
#Add-AppxPackage "https://distro.stackmuncher.com/msix/stackmuncher_x64.msix"

# install from a downloaded package
#Add-AppxPackage stackmuncher_x64.msix 

# check if the package is installed
get-appxpackage |  Where-Object { $_.Name -like "*stackmuncher*" }
