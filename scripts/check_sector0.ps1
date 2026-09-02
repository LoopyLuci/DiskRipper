$ErrorActionPreference = 'Stop'
$rawPath = '\\.\D:'

# Open raw device
$stream = [System.IO.File]::Open($rawPath, 'Open', 'Read', 'ReadWrite')
$buffer = [System.Collections.Generic.List[byte]]::new()
$tempBuf = New-Object byte[4096]
$totalRead = 0
while ($totalRead -lt 2048) {
    $read = $stream.Read($tempBuf, 0, [Math]::Min(4096, 2048 - $totalRead))
    if ($read -eq 0) { break }
    for ($i = 0; $i -lt $read; $i++) {
        $buffer.Add($tempBuf[$i]) | Out-Null
    }
    $totalRead += $read
}
$stream.Close()

# Show first 64 bytes as hex
$hex = ($buffer[0..63] | ForEach-Object { '{0:X2}' -f $_ }) -join ' '
Write-Host "First sector: $hex"

# Check for CD001 at various offsets
$offsets = @(16, 0x8001, 0x1FE, 0x1F8, 1, 0x8000)
foreach ($off in $offsets) {
    if ($off + 5 -le $buffer.Count) {
        $sig = [System.Text.Encoding]::ASCII.GetString($buffer[$off..($off+4)])
        Write-Host "Offset 0x{0:X4}: $sig" -f $off
    }
}
