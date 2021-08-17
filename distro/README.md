# StackMuncher PPA

StackMuncher apps are distributed via [https://distro.stackmuncher.com](https://distro.stackmuncher.com).

That domain name is a mirror of *distro* folder, where this README is located, except that the executables are not committed to the repo.
They are uploaded straight into an S3 bucket and is accessed via CloudFront.

## Folder contents

* **ubuntu**: everything needed for the PPA to work with `apt` command, except for *.deb* files that are sent straight to S3 to avoid clogging the repo
* **msix**: everything needed for Windows packaging and distribution 
* **create_ppa.sh**: use it to re-create the PPA or some parts of it, e.g. if the domain or the key change
* **update_linux.sh**: run it from the root folder of the project to create a new package and upload it to S3 

# Build system for StackMuncher app

All *.sh* and *.ps1* scripts should be run from the project root.

## Linux

The app should be deployed to `/usr/bin/stackmuncher` and the rules to `/usr/share/stackmuncher/stm_rules/`. The choice is based on [Linux Filesystem Hierarchy Standard](https://www.pathname.com/fhs/).

### deb package

deb builds rely on [cargo-deb](https://crates.io/crates/cargo-deb) crate. See config inside [stackmuncher/Cargo.toml](stackmuncher/Cargo.toml) `[package.metadata.deb]` table.
* build deb: `cargo deb -p stackmuncher`
* bump the patch #: `cargo semver bump patch`
* package for PPA: `. ppa/update.sh`
* inspect deb: `dpkg-deb -x /home/ubuntu/rust/stackmuncher/target/debian/stackmuncher_0.1.0_amd64.deb .`
* about deb packages: https://blog.packagecloud.io/eng/2015/10/13/inspect-extract-contents-debian-packages/

## Windows

The current MSIX config produces a *full trust* Windows App, which should be *partial trust* with only local folder access. See https://github.com/stackmuncher/stm/issues/11

Windows building and packaging steps:

1. Build all targets: *distro\msix\build_win_all.ps1*
2. Update the build number in MSIX for the installer to recognize it as a new version: *distro\msix\bump_msix_build_number.ps1*
3. Package: *distro\msix\msix_generate.ps1*
4. Upload to S3: *distro\msix\upload_win_all.ps1*

Installers with auto-update:
* http://distro.stackmuncher.com/msix/stackmuncher_x64.appinstaller
* http://distro.stackmuncher.com/msix/stackmuncher_arm64.appinstaller

Installers without auto-update:
* http://distro.stackmuncher.com/msix/stackmuncher_x64.msix
* http://distro.stackmuncher.com/msix/stackmuncher_arm64.msix

PowerShell installer script:
* http://distro.stackmuncher.com/msix/msix_install_locally.ps1

Installation process:
* An up-to-date Windows 10: download and click on the package file
* Windows Server 2019, older Win10 or a manual install: use the PS script. 

Dev cert: http://distro.stackmuncher.com/msix/stm_dev.cer <-- FOR DEVELOPMENT ONLY

#### Dev and testing

*stm_dev.pfx* is needed to sign a package. *stm_dev.cer* should be installed on the test machine. Both files can be generated with *distro\msix\cert_gen_self_signed.ps1*.
Use *distro\msix\cert_add_self_signed_to_root.ps1* to install it as *root* or click on the file (Win10 only).

### Build server set up

This is an outline of the manual set up. Most of it can be scripted later.

* Create an instance with AWS Win Base AMI
* Disable Enhanced Sec config in Server manager
* Install FireFox https://www.mozilla.org/en-US/firefox/download/thanks/
* Install VS Build Tools https://visualstudio.microsoft.com/visual-cpp-build-tools/
  * C++ Build Tools Workload
  * EN language pack
  * MSVC NS2019 C++ x64/86 Build Tools
  * MSVC NS2019 C++ ARM64 Build Tools
  * MSVC NS2019 C++ ARM64 Spectre Build Tools
  * Win 10 SDK
  * VS SDK Build Tools Core
  * whatever else is selected by default for this workload
* Install Rust https://www.rust-lang.org/tools/install?platform_override=win
* Install Git https://git-scm.com/download/win
* Install AWS CLI https://docs.aws.amazon.com/cli/latest/userguide/install-cliv2-windows.html#cliv2-windows-install
* Install VSCode https://code.visualstudio.com/download
* Run `rustup target add aarch64-pc-windows-msvc`
* Make sure `cl.exe` is included in PATH, e.g. `C:\Program Files (x86)\Microsoft Visual Studio\2019\BuildTools\VC\Tools\MSVC\14.28.29910\bin\Hostx64\x64\`


#### Related resources

* MSIX package schema: https://docs.microsoft.com/en-us/uwp/schemas/appxpackage/appx-package-manifest
* MSIX app installer schema: https://docs.microsoft.com/en-us/uwp/schemas/appinstallerschema/schema-root
* MSIX auto-update: https://docs.microsoft.com/en-us/uwp/schemas/appinstallerschema/element-update-settings
* More on MSIX auto-update: https://techcommunity.microsoft.com/t5/windows-dev-appconsult/handling-application-updates-with-app-installer-and-msix-in/ba-p/355389
* MSIX manual packaging https://docs.microsoft.com/en-us/windows/msix/package/manual-packaging-root
* MSIX install with PS: https://docs.microsoft.com/en-us/powershell/module/appx/add-appxpackage?view=windowsserver2019-ps#parameters

----
_This section was copied from the root README file._

# Installation instructions

StackMuncher is a single executable file written in Rust. Its only external dependencies are `git` and JSON files with stack analysis templates.

Run StackMuncher client app from the root of your project with a child `.git` folder.
The app will access the contents of the repository and save its stack analysis reports in temporary folder.

## Ubuntu

```shell
curl -SsL https://distro.stackmuncher.com/ubuntu/KEY.gpg | sudo apt-key add -
sudo curl -SsL -o /etc/apt/sources.list.d/stackmuncher.list https://distro.stackmuncher.com/ubuntu/stackmuncher.list
sudo apt update
sudo apt install stackmuncher
```

To uninstall everything run:
```shell
sudo apt remove stackmuncher
sudo apt-key del AC98A3AC
sudo rm /etc/apt/sources.list.d/stackmuncher.list
```

## Windows

Download and run the installer from https://distro.stackmuncher.com/msix/stackmuncher_x64.appinstaller or use this PowerShell command:

```powershell
Add-AppxPackage "https://distro.stackmuncher.com/msix/stackmuncher_x64.appinstaller" -AppInstallerFile
```

To uninstall everything run this PowerShell command: `Get-AppxPackage |  Where-Object { $_.Name -like "*stackmuncher*" } | Remove-AppxPackage`

See [distro](distro) section for more detailed installation instructions and troubleshooting.
