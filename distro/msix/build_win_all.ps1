# builds different windows targets for release and updates deployment packages
# must be run from the root of the project

#cargo build --target x86_64-pc-windows-msvc
cargo build --release --target x86_64-pc-windows-msvc
cargo build --release --target aarch64-pc-windows-msvc

#application/msix, application/appinstaller
# https://awscli.amazonaws.com/v2/documentation/api/latest/reference/s3/cp.html