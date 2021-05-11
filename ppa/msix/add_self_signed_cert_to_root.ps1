Get-ChildItem -path "Cert:\LocalMachine\Root" | Where-Object { $_.Subject -like "*stackmuncher*" } | Remove-Item
Import-Certificate -FilePath ppa\stm_test.cer  -CertStoreLocation Cert:\LocalMachine\Root
