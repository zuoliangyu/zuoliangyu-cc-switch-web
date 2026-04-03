param(
  [ValidateSet('all', 'w', 'l', 'd')]
  [string]$Mode = 'all'
)

$ErrorActionPreference = 'Stop'

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$repoRoot = (Resolve-Path (Join-Path $scriptDir '..')).Path
$outputRoot = Join-Path $repoRoot 'release\local-artifacts'
$windowsOutputDir = Join-Path $outputRoot 'windows'
$linuxOutputDir = Join-Path $outputRoot 'linux'
$dockerOutputDir = Join-Path $outputRoot 'docker'
$windowsBinary = Join-Path $repoRoot 'backend\target\release\cc-switch-web.exe'
$windowsArtifact = Join-Path $windowsOutputDir 'cc-switch-web.exe'
$linuxTempDir = Join-Path $linuxOutputDir 'buildx-output'
$linuxArtifact = Join-Path $linuxOutputDir 'cc-switch-web-linux-x64.tar.gz'
$dockerArtifactTar = Join-Path $dockerOutputDir 'cc-switch-web-docker-image.tar'
$dockerArtifactGz = Join-Path $dockerOutputDir 'cc-switch-web-docker-image.tar.gz'

function Show-Usage {
  Write-Host 'Usage: .\scripts\package-artifacts.ps1 [all|w|l|d]'
  Write-Host '  all: 导出 Windows / Linux / Docker 全部产物（默认）'
  Write-Host '  w:   仅导出 Windows 可执行文件'
  Write-Host '  l:   仅导出 Linux 发布包'
  Write-Host '  d:   仅导出 Docker 镜像包'
}

function New-CleanDirectory {
  param([string]$Path)

  if (Test-Path $Path) {
    Remove-Item -LiteralPath $Path -Recurse -Force
  }

  New-Item -ItemType Directory -Path $Path | Out-Null
}

function Ensure-Directory {
  param([string]$Path)

  if (-not (Test-Path $Path)) {
    New-Item -ItemType Directory -Path $Path | Out-Null
  }
}

function Invoke-Step {
  param(
    [string]$Name,
    [scriptblock]$Action
  )

  Write-Host ('package: {0}' -f $Name)
  & $Action
  if ($LASTEXITCODE -ne 0) {
    throw ("Step failed: {0}" -f $Name)
  }
}

function Compress-GzipFile {
  param(
    [string]$SourcePath,
    [string]$DestinationPath
  )

  Add-Type -AssemblyName 'System.IO.Compression.FileSystem'

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
  Ensure-Directory $outputRoot

  $runWindows = $Mode -in @('all', 'w')
  $runLinux = $Mode -in @('all', 'l')
  $runDocker = $Mode -in @('all', 'd')

  if (-not ($runWindows -or $runLinux -or $runDocker)) {
    Show-Usage
    throw ('Unsupported mode: ' + $Mode)
  }

  if ($runWindows) {
    New-CleanDirectory $windowsOutputDir

    Invoke-Step 'Building frontend bundle for Windows artifact' {
      & pnpm exec vite build
    }

    Invoke-Step 'Building Windows release binary' {
      & cargo build --locked --release --manifest-path 'backend/Cargo.toml' --bin 'cc-switch-web'
    }

    if (-not (Test-Path $windowsBinary)) {
      throw ('Windows binary not found: ' + $windowsBinary)
    }
    Copy-Item -LiteralPath $windowsBinary -Destination $windowsArtifact -Force
  }

  if ($runLinux) {
    New-CleanDirectory $linuxOutputDir
    New-CleanDirectory $linuxTempDir

    Invoke-Step 'Exporting Linux musl package with Docker Buildx' {
      $dockerArgs = @(
        'buildx',
        'build',
        '--target',
        'package-linux-tar',
        '--output',
        ('type=local,dest=' + $linuxTempDir),
        '.'
      )
      & docker @dockerArgs
    }

    $linuxCandidates = @(
      (Join-Path $linuxTempDir 'cc-switch-web-linux-x64.tar.gz'),
      (Join-Path $linuxTempDir 'out\cc-switch-web-linux-x64.tar.gz')
    )
    $linuxSource = $linuxCandidates | Where-Object { Test-Path $_ } | Select-Object -First 1
    if (-not $linuxSource) {
      throw ('Linux package export not found under ' + $linuxTempDir)
    }
    Copy-Item -LiteralPath $linuxSource -Destination $linuxArtifact -Force
    Remove-Item -LiteralPath $linuxTempDir -Recurse -Force
  }

  if ($runDocker) {
    New-CleanDirectory $dockerOutputDir

    Invoke-Step 'Exporting Docker image tar with Docker Buildx' {
      $dockerArgs = @(
        'buildx',
        'build',
        '--tag',
        'cc-switch-web:local',
        '--output',
        ('type=docker,dest=' + $dockerArtifactTar),
        '.'
      )
      & docker @dockerArgs
    }

    Compress-GzipFile -SourcePath $dockerArtifactTar -DestinationPath $dockerArtifactGz
    Remove-Item -LiteralPath $dockerArtifactTar -Force
  }

  Write-Host ''
  Write-Host ('package: done (mode={0})' -f $Mode)
  if ($runWindows) {
    Write-Host ('package: windows artifact: {0}' -f $windowsArtifact)
  }
  if ($runLinux) {
    Write-Host ('package: linux artifact:   {0}' -f $linuxArtifact)
  }
  if ($runDocker) {
    Write-Host ('package: docker artifact:  {0}' -f $dockerArtifactGz)
    Write-Host 'package: docker load:      docker load -i' $dockerArtifactGz
  }
} finally {
  Pop-Location
}
