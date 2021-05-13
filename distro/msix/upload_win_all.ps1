# uploads windows deployment packages to S3
# must be run from the root of the project

# auxilliary files
aws s3 cp distro\msix\stm_dev.cer s3://stm-ppa-7b4g14ydlm/msix/stm_dev.cer
aws s3 cp distro\stm_rules.zip s3://stm-ppa-7b4g14ydlm/stm_rules.zip
aws s3 cp distro\msix\msix_install_locally.ps1 s3://stm-ppa-7b4g14ydlm/msix/msix_install_locally.ps1

# installer packages
aws s3 cp distro\msix\stackmuncher_x64.msix s3://stm-ppa-7b4g14ydlm/msix/stackmuncher_x64.msix --content-type "application/msix"
aws s3 cp distro\msix\stackmuncher_x64.appinstaller s3://stm-ppa-7b4g14ydlm/msix/stackmuncher_x64.appinstaller --content-type "application/appinstaller"
aws s3 cp distro\msix\stackmuncher_x64.msix s3://stm-ppa-7b4g14ydlm/msix/stackmuncher_arm64.msix --content-type "application/msix"
aws s3 cp distro\msix\stackmuncher_x64.appinstaller s3://stm-ppa-7b4g14ydlm/msix/stackmuncher_arm64.appinstaller --content-type "application/appinstaller"

# executables
aws s3 cp target\x86_64-pc-windows-msvc\release\stackmuncher.exe s3://stm-ppa-7b4g14ydlm/target/x86_64-pc-windows-msvc/release/stackmuncher.exe
aws s3 cp target\aarch64-pc-windows-msvc\release\stackmuncher.exe s3://stm-ppa-7b4g14ydlm/target/aarch64-pc-windows-msvc/release/stackmuncher.exe

# index update
echo "This domain is used to distribute StackMuncher software packages for different platforms. Learn more from https://github.com/stackmuncher/stm/distro." > distro\index.txt
echo "" >> distro\index.txt
aws s3 ls s3://stm-ppa-7b4g14ydlm/ --recursive --summarize >> distro\index.txt
aws s3 cp distro\index.txt s3://stm-ppa-7b4g14ydlm/index.txt
aws cloudfront create-invalidation --distribution-id E102XVLT2KLJHS --paths "/"
