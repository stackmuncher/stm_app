# To be used in development only

Get-ChildItem -path "Cert:\LocalMachine\Root" | Where-Object { $_.Subject -like "*stackmuncher*" } | Remove-Item
Import-Certificate -FilePath distro\msix\stm_dev.cer  -CertStoreLocation Cert:\LocalMachine\Root
