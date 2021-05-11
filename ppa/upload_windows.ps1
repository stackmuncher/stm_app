# uploads windows deployment packages to S3
# must be run from the root of the project

aws s3 cp ppa\stm_test.cer s3://stm-ppa-7b4g14ydlm/stm_test.cer

aws s3 cp ppa\msix\stackmuncher.msix s3://stm-ppa-7b4g14ydlm/msix/stackmuncher.msix --content-type "application/msix"
aws s3 cp ppa\msix\stackmuncher.appinstaller s3://stm-ppa-7b4g14ydlm/msix/stackmuncher.appinstaller --content-type "application/appinstaller"

# cargo build --release --target x86_64-pc-windows-msvc
aws s3 cp target\x86_64-pc-windows-msvc\release\stackmuncher.exe s3://stm-ppa-7b4g14ydlm/target/x86_64-pc-windows-msvc/release/stackmuncher.exe
aws s3 cp target\x86_64-pc-windows-msvc\release\stackmuncher.pdb s3://stm-ppa-7b4g14ydlm/target/x86_64-pc-windows-msvc/release/stackmuncher.pdb

# cargo build --release --target aarch64-pc-windows-msvc
aws s3 cp target\aarch64-pc-windows-msvc\release\stackmuncher.exe s3://stm-ppa-7b4g14ydlm/target/aarch64-pc-windows-msvc/release/stackmuncher.exe
aws s3 cp target\aarch64-pc-windows-msvc\release\stackmuncher.pdb s3://stm-ppa-7b4g14ydlm/target/aarch64-pc-windows-msvc/release/stackmuncher.pdb

echo "This domain is used to distribute StackMuncher software packages for different platforms. Learn more from https://github.com/stackmuncher/stm/ppa." > ppa\index.txt
echo "" >> ppa\index.txt
aws s3 ls s3://stm-ppa-7b4g14ydlm/ --recursive --summarize >> ppa\index.txt
aws s3 cp ppa\index.txt s3://stm-ppa-7b4g14ydlm/index.txt
aws cloudfront create-invalidation --distribution-id E102XVLT2KLJHS --paths "/"
