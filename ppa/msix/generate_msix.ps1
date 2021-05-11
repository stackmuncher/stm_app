if (test-path ppa\msix\content\priconfig.xml) {rm ppa\msix\content\priconfig.xml}
if (test-path ppa\msix\content\resources.pri) {rm ppa\msix\content\resources.pri}
if (test-path ppa\msix\stackmuncher.msix) {rm ppa\msix\stackmuncher.msix}
if (test-path ppa\msix\content\stackmuncher.exe) {rm ppa\msix\content\stackmuncher.exe}
if (test-path ppa\msix\content\stm_rules) {rm ppa\msix\content\stm_rules -recurse}
cp target\release\stackmuncher.exe ppa\msix\content\
cp stm_rules\file_types ppa\msix\content\stm_rules\file_types -recurse
cp stm_rules\file_types ppa\msix\content\stm_rules\munchers -recurse
cd ppa\msix\content; makepri.exe createconfig /cf priconfig.xml /dq en-US; makepri.exe new /pr . /cf priconfig.xml
cd ..\..\..
MakeAppx pack /v /o /d ppa\msix\content /p ppa\msix\stackmuncher.msix
signtool sign /v /fd SHA256 /a /f ppa\stm_test.pfx /p "123" -t http://timestamp.digicert.com ppa\msix\stackmuncher.msix


# test-install the package
# Add-AppxPackage -Register ppa\msix\content\appxmanifest.xml
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