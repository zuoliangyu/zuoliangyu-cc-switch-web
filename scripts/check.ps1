param()

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$repoRoot = (Resolve-Path (Join-Path $scriptDir "..")).Path

Push-Location $repoRoot
try {
  & node (Join-Path $scriptDir "check.mjs")
  exit $LASTEXITCODE
} finally {
  Pop-Location
}
