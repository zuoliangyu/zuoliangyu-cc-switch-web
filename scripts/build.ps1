param(
  [ValidateSet("w", "d")]
  [string]$Mode = "w"
)

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$repoRoot = (Resolve-Path (Join-Path $scriptDir "..")).Path

Push-Location $repoRoot
try {
  & node (Join-Path $scriptDir "build.mjs") $Mode
  exit $LASTEXITCODE
} finally {
  Pop-Location
}
