param()

$ErrorActionPreference = "Stop"

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$repoRoot = (Resolve-Path (Join-Path $scriptDir "..")).Path
$outputRoot = Join-Path $repoRoot "release\local-artifacts"
$windowsOutputDir = Join-Path $outputRoot "windows"
$linuxOutputDir = Join-Path $outputRoot "linux"
$dockerOutputDir = Join-Path $outputRoot "docker"
$windowsBinary = Join-Path $repoRoot "backend\target\release\cc-switch-web.exe"
$windowsArtifact = Join-Path $windowsOutputDir "cc-switch-web.exe"
$linuxTempDir = Join-Path $linuxOutputDir "buildx-output"
$linuxArtifact = Join-Path $linuxOutputDir "cc-switch-web-linux-x64.tar.gz"
$dockerArtifactTar = Join-Path $dockerOutputDir "cc-switch-web-docker-image.tar"
$dockerArtifactGz = Join-Path $dockerOutputDir "cc-switch-web-docker-image.tar.gz"

function New-CleanDirectory {
  param([string]$Path)

  if (Test-Path $Path) {
    Remove-Item -LiteralPath $Path -Recurse -Force
  }

  New-Item -ItemType Directory -Path $Path | Out-Null
}

function Invoke-Step {
  param(
    [string]$Name,
    [scriptblock]$Action
  )

  Write-Host "[package] $Name"
  & $Action
  if ($LASTEXITCODE -ne 0) {
    throw "Step failed: $Name"
  }
}

function Compress-GzipFile {
  param(
    [string]$SourcePath,
    [string]$DestinationPath
  )

  Add-Type -AssemblyName System.IO.Compression.FileSystem

  if (Test-Path $DestinationPath) {
    Remove-Item -LiteralPath $DestinationPath -Force
  }

  $inputStream = [System.IO.File]::OpenRead($SourcePath)
  try {
    $outputStream = [System.IO.File]::Create($DestinationPath)
    try {
      $gzipStream = New-Object System.IO.Compression.GzipStream(
        $outputStream,
        [System.IO.Compression.CompressionLevel]::Optimal
      )
      try {
        $inputStream.CopyTo($gzipStream)
      } finally {
        $gzipStream.Dispose()
      }
    } finally {
      $outputStream.Dispose()
    }
  } finally {
    $inputStream.Dispose()
  }
}

Push-Location $repoRoot
try {
  New-CleanDirectory $outputRoot
  New-CleanDirectory $windowsOutputDir
  New-CleanDirectory $linuxOutputDir
  New-CleanDirectory $dockerOutputDir

  Invoke-Step "Building frontend bundle" {
    & pnpm exec vite build
  }

  Invoke-Step "Building Windows release binary" {
    & cargo build --locked --release --manifest-path backend/Cargo.toml --bin cc-switch-web
  }

  if (-not (Test-Path $windowsBinary)) {
    throw "Windows binary not found: $windowsBinary"
  }
  Copy-Item -LiteralPath $windowsBinary -Destination $windowsArtifact -Force

  New-CleanDirectory $linuxTempDir
  Invoke-Step "Exporting Linux musl package with Docker Buildx" {
    & docker buildx build `
      --target package-linux-tar `
      --output "type=local,dest=$linuxTempDir" `
      .
  }

  $linuxCandidates = @(
    (Join-Path $linuxTempDir "cc-switch-web-linux-x64.tar.gz"),
    (Join-Path $linuxTempDir "out\cc-switch-web-linux-x64.tar.gz")
  )
  $linuxSource = $linuxCandidates | Where-Object { Test-Path $_ } | Select-Object -First 1
  if (-not $linuxSource) {
    throw "Linux package export not found under $linuxTempDir"
  }
  Copy-Item -LiteralPath $linuxSource -Destination $linuxArtifact -Force
  Remove-Item -LiteralPath $linuxTempDir -Recurse -Force

  Invoke-Step "Exporting Docker image tar with Docker Buildx" {
    & docker buildx build `
      --tag cc-switch-web:local `
      --output "type=docker,dest=$dockerArtifactTar" `
      .
  }

  Compress-GzipFile -SourcePath $dockerArtifactTar -DestinationPath $dockerArtifactGz
  Remove-Item -LiteralPath $dockerArtifactTar -Force

  Write-Host ""
  Write-Host "[package] done"
  Write-Host "[package] windows artifact: $windowsArtifact"
  Write-Host "[package] linux artifact:   $linuxArtifact"
  Write-Host "[package] docker artifact:  $dockerArtifactGz"
  Write-Host "[package] docker load:      docker load -i `"$dockerArtifactGz`""
} finally {
  Pop-Location
}
