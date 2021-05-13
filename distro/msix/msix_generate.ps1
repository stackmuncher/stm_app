# Generate MSIX packages for all targets

# clean up x64 manifest 
if (test-path distro\msix\manifest_x64\priconfig.xml) {rm distro\msix\manifest_x64\priconfig.xml}
if (test-path distro\msix\manifest_x64\resources.pri) {rm distro\msix\manifest_x64\resources.pri}
if (test-path distro\msix\stackmuncher_x64.msix) {rm distro\msix\stackmuncher_x64.msix}
if (test-path distro\msix\manifest_x64\stackmuncher.exe) {rm distro\msix\manifest_x64\stackmuncher.exe}
if (test-path distro\msix\manifest_x64\stm_rules) {rm distro\msix\manifest_x64\stm_rules -recurse}

# collect the latest files for packaging
cp target\x86_64-pc-windows-msvc\release\stackmuncher.exe distro\msix\manifest_x64\
cp stm_rules\file_types distro\msix\manifest_x64\stm_rules\file_types -recurse
cp stm_rules\munchers distro\msix\manifest_x64\stm_rules\munchers -recurse
Compress-Archive -Path distro\msix\manifest_x64\stm_rules -DestinationPath distro\stm_rules.zip -Force

cd distro\msix\manifest_x64; makepri.exe createconfig /cf priconfig.xml /dq en-US; makepri.exe new /pr . /cf priconfig.xml
cd ..\..\..
MakeAppx pack /v /o /d distro\msix\manifest_x64 /p distro\msix\stackmuncher_x64.msix
signtool sign /v /fd SHA256 /a /f distro\msix\stm_dev.pfx /p "123" -t http://timestamp.digicert.com distro\msix\stackmuncher_x64.msix


# clean up arm64 manifest 
if (test-path distro\msix\manifest_arm64\priconfig.xml) {rm distro\msix\manifest_arm64\priconfig.xml}
if (test-path distro\msix\manifest_arm64\resources.pri) {rm distro\msix\manifest_arm64\resources.pri}
if (test-path distro\msix\stackmuncher_arm64.msix) {rm distro\msix\stackmuncher_arm64.msix}
if (test-path distro\msix\manifest_arm64\stackmuncher.exe) {rm distro\msix\manifest_arm64\stackmuncher.exe}
if (test-path distro\msix\manifest_arm64\stm_rules) {rm distro\msix\manifest_arm64\stm_rules -recurse}

# reuse x64 appxmanifest as a template
(Get-Content -Path distro\msix\manifest_x64\appxmanifest.xml -Raw) -replace "x64", "arm64" | Set-Content -Path distro\msix\manifest_arm64\appxmanifest.xml -Force
(Get-Content -Path distro\msix\stackmuncher_x64.appinstaller -Raw) -replace "x64", "arm64" | Set-Content -Path distro\msix\stackmuncher_arm64.appinstaller -Force

# collect the latest files for packaging
cp target\aarch64-pc-windows-msvc\release\stackmuncher.exe distro\msix\manifest_arm64\
cp stm_rules\file_types distro\msix\manifest_arm64\stm_rules\file_types -recurse
cp stm_rules\munchers distro\msix\manifest_arm64\stm_rules\munchers -recurse

# gen and sign arm64 msix
cd distro\msix\manifest_arm64; makepri.exe createconfig /cf priconfig.xml /dq en-US; makepri.exe new /pr . /cf priconfig.xml
cd ..\..\..
MakeAppx pack /v /o /d distro\msix\manifest_arm64 /p distro\msix\stackmuncher_arm64.msix
signtool sign /v /fd SHA256 /a /f distro\msix\stm_dev.pfx /p "123" -t http://timestamp.digicert.com distro\msix\stackmuncher_arm64.msix
