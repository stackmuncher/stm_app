Get-ChildItem -path "Cert:\LocalMachine\My" | Where-Object { $_.Subject -like "*stackmuncher*" } | Remove-Item

New-SelfSignedCertificate -Type Custom -Subject "CN=stackmuncher, C=NZ" -KeyUsage DigitalSignature -FriendlyName "stackmuncher" -CertStoreLocation "Cert:\LocalMachine\My" -TextExtension @("2.5.29.37={text}1.3.6.1.5.5.7.3.3", "2.5.29.19={text}")
Get-ChildItem -path "Cert:\LocalMachine\My" | Where-Object { $_.Subject -like "*stackmuncher*" } | Export-PfxCertificate -FilePath ppa\stm_test.pfx -Password (ConvertTo-SecureString -string "123" -Force -AsPlainText) -Force
Get-ChildItem -path "Cert:\LocalMachine\My" | Where-Object { $_.Subject -like "*stackmuncher*" } | Export-Certificate -FilePath ppa\stm_test.cer -Force
