$bytes = [System.IO.File]::ReadAllBytes('C:/Projects/DiskRipper/output/game.iso')
$segment = $bytes[0..511]
$hex = ($segment | ForEach-Object { '{0:X2}' -f $_ }) -join ' '
Write-Host $hex
