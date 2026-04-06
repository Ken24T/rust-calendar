param(
    [switch]$Validate,
    [switch]$Uninstall,
    [switch]$Rollback
)

$ErrorActionPreference = 'Stop'

$selectedModes = @($Validate, $Uninstall, $Rollback) | Where-Object { $_ }
if ($selectedModes.Count -gt 1) {
    throw 'Use only one of -Validate, -Uninstall, or -Rollback at a time.'
}

$repoDir = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$binSource = Join-Path $repoDir 'target\release\rust-calendar.exe'
$installDir = Join-Path $env:LOCALAPPDATA 'Programs\Rust Calendar'
$binDestination = Join-Path $installDir 'rust-calendar.exe'
$binBackup = Join-Path $installDir 'rust-calendar.exe.bak'
$startMenuDir = Join-Path $env:APPDATA 'Microsoft\Windows\Start Menu\Programs'
$shortcutPath = Join-Path $startMenuDir 'Rust Calendar.lnk'

function Get-AppVersion {
    $cargoToml = Join-Path $repoDir 'Cargo.toml'
    $inPackage = $false

    foreach ($line in Get-Content -Path $cargoToml) {
        if ($line -match '^\[package\]') {
            $inPackage = $true
            continue
        }

        if ($inPackage -and $line -match '^\[') {
            break
        }

        if ($inPackage -and $line -match '^version\s*=\s*"([^"]+)"') {
            return $Matches[1]
        }
    }

    return 'unknown'
}

function New-StartMenuShortcut {
    $shell = New-Object -ComObject WScript.Shell
    $shortcut = $shell.CreateShortcut($shortcutPath)
    $shortcut.TargetPath = $binDestination
    $shortcut.WorkingDirectory = $installDir
    $shortcut.IconLocation = $binDestination
    $shortcut.Save()
}

if ($Validate) {
    if (-not (Test-Path $binDestination)) {
        throw "Installed executable not found at $binDestination"
    }

    if (-not (Test-Path $shortcutPath)) {
        throw "Start Menu shortcut not found at $shortcutPath"
    }

    Write-Host 'Windows install validation passed.'
    Write-Host "  Executable -> $binDestination"
    Write-Host "  Shortcut   -> $shortcutPath"
    exit 0
}

if ($Uninstall) {
    Write-Host 'Uninstalling Rust Calendar from the current Windows user profile...'
    Remove-Item $shortcutPath -Force -ErrorAction SilentlyContinue
    Remove-Item $binDestination -Force -ErrorAction SilentlyContinue
    Remove-Item $binBackup -Force -ErrorAction SilentlyContinue

    if (Test-Path $installDir) {
        Remove-Item $installDir -Force -ErrorAction SilentlyContinue
    }

    Write-Host 'Done. Rust Calendar has been removed from this user profile.'
    exit 0
}

if ($Rollback) {
    if (-not (Test-Path $binBackup)) {
        throw "Backup executable not found at $binBackup"
    }

    New-Item -ItemType Directory -Path $installDir -Force | Out-Null
    Copy-Item $binBackup $binDestination -Force
    New-StartMenuShortcut

    Write-Host 'Rollback completed.'
    Write-Host "  Restored -> $binDestination"
    exit 0
}

if (-not (Test-Path $binSource)) {
    throw "Release binary not found at $binSource. Run 'cargo build --release' first."
}

$version = Get-AppVersion

Write-Host "Installing Rust Calendar v$version for the current Windows user..."

New-Item -ItemType Directory -Path $installDir -Force | Out-Null
New-Item -ItemType Directory -Path $startMenuDir -Force | Out-Null

if (Test-Path $binDestination) {
    Copy-Item $binDestination $binBackup -Force
    Write-Host "  Backup   -> $binBackup"
}

Copy-Item $binSource $binDestination -Force
New-StartMenuShortcut

Write-Host "  Binary   -> $binDestination"
Write-Host "  Shortcut -> $shortcutPath"
Write-Host ''
Write-Host 'Rust Calendar installed successfully.'
Write-Host 'Launch it from the Start Menu or run the installed executable directly.'
Write-Host ''
Write-Host 'To validate: powershell -ExecutionPolicy Bypass -File .\packaging\install-windows.ps1 -Validate'
Write-Host 'To uninstall: powershell -ExecutionPolicy Bypass -File .\packaging\install-windows.ps1 -Uninstall'
Write-Host 'To rollback:  powershell -ExecutionPolicy Bypass -File .\packaging\install-windows.ps1 -Rollback'