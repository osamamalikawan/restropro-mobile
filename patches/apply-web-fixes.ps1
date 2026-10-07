# Run from the root of the WEB (Next.js) repo:  .\apply-web-fixes.ps1
# Appends mobile table-scroll rules to globals.css. Safe to run twice.
param([string]$Css = "")

if (-not $Css) {
  $candidates = @("app\globals.css","src\app\globals.css","styles\globals.css","src\styles\globals.css")
  $Css = $candidates | Where-Object { Test-Path $_ } | Select-Object -First 1
}
if (-not $Css -or -not (Test-Path $Css)) {
  Write-Host "globals.css not found. Run again with: .\apply-web-fixes.ps1 -Css path\to\globals.css"
  exit 1
}

$marker = "/* restropro-mobile-fixes */"
if ((Get-Content $Css -Raw) -match [regex]::Escape($marker)) {
  Write-Host "Already applied to $Css"
  exit 0
}

$block = @'

/* restropro-mobile-fixes */
@media (max-width: 768px) {
  /* tables scroll sideways instead of being clipped */
  table {
    display: block;
    max-width: 100%;
    overflow-x: auto;
    white-space: nowrap;
    -webkit-overflow-scrolling: touch;
    touch-action: pan-x pan-y;
  }
  /* let flex/grid parents shrink so the table can scroll inside them */
  main, section, article { min-width: 0; }
}
/* end restropro-mobile-fixes */
'@

Add-Content -Path $Css -Value $block -Encoding UTF8
Write-Host "Patched $Css. Commit, push and redeploy to Vercel."
