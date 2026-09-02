param([switch]$SkipTauri)

$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
$source = Join-Path $projectRoot 'assets\branding\nodesweep-icon.png'
$tauriIcons = Join-Path $projectRoot 'src-tauri\icons'
$public = Join-Path $projectRoot 'client\public'

if (-not (Test-Path -LiteralPath $source)) { throw "Canonical icon not found: $source" }
New-Item -ItemType Directory -Force -Path $tauriIcons, $public | Out-Null

if (-not $SkipTauri) {
  Push-Location $projectRoot
  try { & npm.cmd exec tauri icon $source; if ($LASTEXITCODE -ne 0) { throw 'Tauri icon generation failed.' } }
  finally { Pop-Location }
}

Copy-Item -LiteralPath (Join-Path $tauriIcons '128x128@2x.png') -Destination (Join-Path $public 'nodesweep-icon.png') -Force
Copy-Item -LiteralPath (Join-Path $tauriIcons '32x32.png') -Destination (Join-Path $public 'favicon-32x32.png') -Force
Copy-Item -LiteralPath (Join-Path $tauriIcons 'icon.ico') -Destination (Join-Path $public 'favicon.ico') -Force

Add-Type -AssemblyName System.Drawing
$inputImage = [System.Drawing.Image]::FromFile($source)
try {
  $favicon = New-Object System.Drawing.Bitmap 16, 16
  try {
    $graphics = [System.Drawing.Graphics]::FromImage($favicon)
    try {
      $graphics.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
      $graphics.DrawImage($inputImage, 0, 0, 16, 16)
      $favicon.Save((Join-Path $public 'favicon-16x16.png'), [System.Drawing.Imaging.ImageFormat]::Png)
    } finally { $graphics.Dispose() }
  } finally { $favicon.Dispose() }
} finally { $inputImage.Dispose() }

Write-Host 'NodeSweep branding derivatives generated from assets/branding/nodesweep-icon.png.'
