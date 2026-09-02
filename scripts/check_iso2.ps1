# Check if game.iso has valid ISO 9660 structure
$ErrorActionPreference = 'Stop'
$isoPath = 'C:/Projects/DiskRipper/output/game.iso'
$bytes = [System.IO.File]::ReadAllBytes($isoPath)
Write-Host "File size: $($bytes.Length) bytes"

# Check for ISO 9660 magic at offset 0x8000 (sector 64 + 0x100)
$pvdOffset = 0x8001
if ($bytes.Length -gt $pvdOffset + 6) {
    $magic = [System.Text.Encoding]::ASCII.GetString($bytes[$pvdOffset..($pvdOffset+5)])
    Write-Host "Magic at 0x8001: $magic"
}

# Check for CD001 signature at offset 0x8001
$cd001Offset = 0x8001
if ($bytes.Length -gt $cd001Offset + 5) {
    $sig = [System.Text.Encoding]::ASCII.GetString($bytes[$cd001Offset..($cd001Offset+4)])
    Write-Host "Signature at 0x8001: $sig"
}

# Also check offset 0x1FE for El Torito
$eltoritoOffset = 0x1FE
if ($bytes.Length -gt $eltoritoOffset + 2) {
    $sig = [System.Text.Encoding]::ASCII.GetString($bytes[($eltoritoOffset+1)..($eltoritoOffset+2)])
    Write-Host "At 0x1FF: $sig"
}
