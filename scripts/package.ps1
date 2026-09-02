# DiskRipper Packaging Script for Windows
# Builds distributable packages for Windows (MSI and NSIS)

param(
    [string]$Configuration = "Release",
    [string]$Version = ""
)

$ErrorActionPreference = "Stop"

Write-Host "=== DiskRipper Windows Packaging ===" -ForegroundColor Green
Write-Host ""

# Configuration
$AppName = "DiskRipper"
$ProjectRoot = Split-Path -Parent $PSScriptRoot
$FrontendDir = Join-Path $ProjectRoot "frontend"
$TauriDir = Join-Path $ProjectRoot "diskripper-tauri"
$OutputDir = Join-Path $ProjectRoot "packages"

# Get version from Cargo.toml if not specified
if ([string]::IsNullOrEmpty($Version)) {
    $CargoToml = Join-Path $ProjectRoot "Cargo.toml"
    $VersionLine = Get-Content $CargoToml | Where-Object { $_ -match '^version' } | Select-Object -First 1
    $Version = ($VersionLine -split '"')[1]
}

Write-Host "Building $AppName v$Version" -ForegroundColor Cyan
Write-Host ""

# Create output directory
New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null

# Step 1: Build frontend
Write-Host "Step 1: Building frontend..." -ForegroundColor Yellow
Set-Location $FrontendDir
npm install
npm run build
Set-Location $TauriDir

# Step 2: Build Rust backend
Write-Host "Step 2: Building Rust backend..." -ForegroundColor Yellow
cargo build --release

# Step 3: Create icons if they don't exist
Write-Host "Step 3: Checking icons..." -ForegroundColor Yellow
$IconsDir = Join-Path $TauriDir "icons"
New-Item -ItemType Directory -Force -Path $IconsDir | Out-Null

$RequiredIcons = @(
    "32x32.png",
    "128x128.png",
    "128x128@2x.png",
    "icon.png",
    "icon.ico",
    "icon.icns"
)

foreach ($icon in $RequiredIcons) {
    $iconPath = Join-Path $IconsDir $icon
    if (-not (Test-Path $iconPath)) {
        Write-Host "  Missing: $icon (will be created as placeholder)" -ForegroundColor DarkYellow
        # Create a simple 1x1 pixel PNG as placeholder
        $bytes = [byte[]]@(0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A)
        [System.IO.File]::WriteAllBytes($iconPath, $bytes)
    }
}

# Step 4: Build Tauri app
Write-Host "Step 4: Building Tauri application..." -ForegroundColor Yellow
cargo tauri build

# Step 5: Copy packages to output directory
Write-Host "Step 5: Copying packages..." -ForegroundColor Yellow
$BundleDir = Join-Path $TauriDir "src-tauri\target\release\bundle"
if (Test-Path $BundleDir) {
    Copy-Item -Path "$BundleDir\*" -Destination $OutputDir -Recurse -Force
}

# Step 6: Create checksums
Write-Host "Step 6: Creating checksums..." -ForegroundColor Yellow
$ChecksumFile = Join-Path $OutputDir "checksums.txt"
Get-ChildItem -Path $OutputDir -Include *.msi,*.exe,*.zip -Recurse | ForEach-Object {
    $hash = Get-FileHash $_.FullName -Algorithm SHA256
    "$($hash.Hash)  $($_.Name)" | Out-File -Append -FilePath $ChecksumFile
}

Write-Host ""
Write-Host "=== Packaging Complete ===" -ForegroundColor Green
Write-Host ""
Write-Host "Packages available in: $OutputDir" -ForegroundColor Cyan
Write-Host ""
Get-ChildItem -Path $OutputDir -Recurse | Format-Table Name, Length, LastWriteTime
Write-Host ""
Write-Host "Checksums:" -ForegroundColor Cyan
Get-Content $ChecksumFile -ErrorAction SilentlyContinue
