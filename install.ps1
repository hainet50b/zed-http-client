#Requires -Version 5
# Install script for httpc
# Usage: irm https://raw.githubusercontent.com/hainet50b/zed-http-client/main/install.ps1 | iex

$ErrorActionPreference = "Stop"

$Repo = "hainet50b/zed-http-client"
$InstallDir = if ($env:HTTPC_INSTALL_DIR) { $env:HTTPC_INSTALL_DIR } else { "$env:USERPROFILE\.httpc\bin" }

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
    Move-Item -Path (Join-Path $TempDir "httpc.exe") -Destination (Join-Path $InstallDir "httpc.exe") -Force

    Write-Host "Installed httpc to $InstallDir\httpc.exe"

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
        & (Join-Path $InstallDir "httpc.exe") --version
    }
} finally {
    if (Test-Path $TempDir) { Remove-Item $TempDir -Recurse -Force }
}
