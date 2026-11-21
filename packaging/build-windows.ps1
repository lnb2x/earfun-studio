$ErrorActionPreference = "Stop"
$AppDir = Split-Path -Parent $PSScriptRoot
Set-Location $AppDir

$VersionMatch = Select-String -Path Cargo.toml -Pattern '^version = "(\d+\.\d+\.\d+[^"]*)"' | Select-Object -First 1
if (-not $VersionMatch) {
    throw "Could not parse package version from Cargo.toml"
}
$Version = $VersionMatch.Matches[0].Groups[1].Value

cargo build --locked --release
if ($LASTEXITCODE -ne 0) { throw "Cargo build failed" }

$BuiltExe = "$AppDir\target\release\earfun-studio.exe"
if (-not (Test-Path $BuiltExe)) {
    throw "Expected release binary not found at $BuiltExe"
}

New-Item -ItemType Directory -Force -Path "$AppDir\dist" | Out-Null
$StageDir = "$AppDir\target\package-win\EarFunStudio-Windows"
if (Test-Path "$AppDir\target\package-win") {
    Remove-Item -Recurse -Force "$AppDir\target\package-win"
}
New-Item -ItemType Directory -Force -Path $StageDir | Out-Null

$StandaloneExe = "$AppDir\dist\EarFunStudio-Windows-${Version}-x86_64.exe"
Copy-Item $BuiltExe "$StageDir\EarFunStudio.exe"
Copy-Item $BuiltExe $StandaloneExe
Copy-Item "$AppDir\README.md" "$StageDir\README.md"
Copy-Item "$AppDir\LICENSE" "$StageDir\LICENSE"
Copy-Item "$AppDir\assets\fonts\OFL.txt" "$StageDir\OFL-BeVietnamPro.txt"
Copy-Item -Recurse "$AppDir\docs" "$StageDir\docs"

$ZipPath = "$AppDir\dist\EarFunStudio-Windows-${Version}-x86_64.zip"
if (Test-Path $ZipPath) {
    Remove-Item -Force $ZipPath
}
Compress-Archive -Path $StageDir -DestinationPath $ZipPath

Write-Output $ZipPath
Write-Output $StandaloneExe
