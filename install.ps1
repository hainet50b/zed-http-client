#Requires -Version 5
# Install script for httpc
# Usage: irm https://raw.githubusercontent.com/hainet50b/zed-http-client/main/install.ps1 | iex

$ErrorActionPreference = "Stop"

$Repo = "hainet50b/zed-http-client"
$InstallDir = if ($env:HTTPC_INSTALL_DIR) { $env:HTTPC_INSTALL_DIR } else { "$env:USERPROFILE\.httpc\bin" }

function Test-AlreadyLatest {
    if ($env:HTTPC_FORCE_INSTALL) { return }
    if (-not (Get-Command httpc -ErrorAction SilentlyContinue)) { return }
    $LocalVersionLine = & httpc --version 2>$null
    if (-not $LocalVersionLine) { return }
    $LocalVersion = ($LocalVersionLine -split '\s+')[-1]
    if (-not $LocalVersion) { return }
    try {
        $Release = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases/latest" -TimeoutSec 5 -ErrorAction Stop
    } catch {
        return
    }
    $LatestVersion = if ($Release.tag_name) { $Release.tag_name -replace '^v', '' } else { $null }
    if (-not $LatestVersion) { return }
    if ($LocalVersion -eq $LatestVersion) {
        Write-Host "httpc $LocalVersion is already the latest. Set HTTPC_FORCE_INSTALL=1 to reinstall."
        exit 0
    }
}

Test-AlreadyLatest

$Arch = [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture
$Target = switch ($Arch) {
    "X64"   { "x86_64-pc-windows-msvc" }
    "Arm64" { "aarch64-pc-windows-msvc" }
    default { throw "Unsupported architecture: $Arch" }
}

$Url = "https://github.com/$Repo/releases/latest/download/httpc-$Target.zip"

$TempDir = Join-Path ([System.IO.Path]::GetTempPath()) "httpc-install-$([guid]::NewGuid())"
New-Item -ItemType Directory -Path $TempDir -Force | Out-Null
$TempZip = Join-Path $TempDir "httpc.zip"

try {
    Write-Host "Downloading httpc for $Target..."
    Invoke-WebRequest -Uri $Url -OutFile $TempZip -UseBasicParsing

    Write-Host "Extracting..."
    Expand-Archive -Path $TempZip -DestinationPath $TempDir -Force

    if (-not (Test-Path $InstallDir)) {
        New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
    }
    $TargetExe = Join-Path $InstallDir "httpc.exe"
    if (Test-Path $TargetExe) {
        $Suffix = [guid]::NewGuid().ToString("N").Substring(0, 8)
        $OldName = "httpc.exe.old-$Suffix"
        try {
            Rename-Item -Path $TargetExe -NewName $OldName -ErrorAction Stop
        } catch {
            throw "Failed to rename existing $TargetExe. Close any running 'httpc' processes and any terminals where httpc was recently invoked, then re-run the installer. Original error: $_"
        }
    }
    Move-Item -Path (Join-Path $TempDir "httpc.exe") -Destination $TargetExe -Force

    Get-ChildItem -Path $InstallDir -Filter "httpc.exe.old-*" -ErrorAction SilentlyContinue | ForEach-Object {
        try {
            Remove-Item $_.FullName -Force -ErrorAction Stop
        } catch {
            # Still locked by a running process; the next install will retry.
        }
    }

    Write-Host "Installed httpc to $TargetExe"

    $UserPath = [Environment]::GetEnvironmentVariable("Path", "User")
    $PathParts = if ($UserPath) { $UserPath -split ';' | Where-Object { $_ -ne '' } } else { @() }
    if ($PathParts -notcontains $InstallDir) {
        $NewPath = ((@($PathParts) + $InstallDir) -join ';')
        [Environment]::SetEnvironmentVariable("Path", $NewPath, "User")
        Write-Host ""
        Write-Host "Added $InstallDir to your user PATH."
        Write-Host "Open a new terminal to use 'httpc'."
    } else {
        Write-Host ""
        & $TargetExe --version
    }
} finally {
    if (Test-Path $TempDir) { Remove-Item $TempDir -Recurse -Force }
}
