# uploads windows deployment packages to S3 for quick testing across VMs
# must be run from the root of the project in dev environment

aws s3 cp distro\msix\stackmuncher_x64.msix s3://stm-ppa-7b4g14ydlm/msix/stackmuncher_x64.msix --content-type "application/msix"
#aws s3 cp distro\msix\stackmuncher_x64.appinstaller s3://stm-ppa-7b4g14ydlm/msix/stackmuncher_x64.appinstaller --content-type "application/appinstaller"

