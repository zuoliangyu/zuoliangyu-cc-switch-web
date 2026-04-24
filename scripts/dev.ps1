param(
  [ValidateSet("w", "d")]
  [string]$Mode = "w",
  [Parameter(ValueFromRemainingArguments = $true)]
  [string[]]$ExtraArgs
)

. (Join-Path $PSScriptRoot "lib\entry.ps1")

$extras = if ($ExtraArgs) { $ExtraArgs } else { @() }
exit (Invoke-RepoNodeScript -ScriptPath 'scripts/dev.mjs' -Arguments (@($Mode) + $extras))
