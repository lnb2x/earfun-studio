$ErrorActionPreference = "Stop"
$AppDir = Split-Path -Parent $PSScriptRoot
Set-Location $AppDir

$Version = "1.3.0"
cargo build --release

New-Item -ItemType Directory -Force -Path "$AppDir\dist" | Out-Null
$StageDir = "$AppDir\target\package-win\EarFunStudio-Windows"
if (Test-Path "$AppDir\target\package-win") {
    Remove-Item -Recurse -Force "$AppDir\target\package-win"
}
New-Item -ItemType Directory -Force -Path $StageDir | Out-Null

Copy-Item "$AppDir\target\release\earfun-studio.exe" "$StageDir\EarFunStudio.exe"
Copy-Item "$AppDir\target\release\earfun-studio.exe" "$AppDir\dist\EarFunStudio-Windows-${Version}-x86_64.exe"
Copy-Item "$AppDir\README.md" "$StageDir\README.md"
Copy-Item -Recurse "$AppDir\docs" "$StageDir\docs"

$ZipPath = "$AppDir\dist\EarFunStudio-Windows-${Version}-x86_64.zip"
if (Test-Path $ZipPath) {
    Remove-Item -Force $ZipPath
}
Compress-Archive -Path $StageDir -DestinationPath $ZipPath
Write-Output $ZipPath
Write-Output "$AppDir\dist\EarFunStudio-Windows-${Version}-x86_64.exe"
