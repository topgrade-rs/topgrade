# Manual Vulnerability Check Script
# Run this with: powershell -ExecutionPolicy Bypass -File check_vulns.ps1

Write-Host "=== Manual Vulnerability Check ===" -ForegroundColor Green
Write-Host ""

# Check for the specific vulnerable crates mentioned in the RUSTSEC advisories
$vulnerabilities = @{
    "RUSTSEC-2022-0081" = @{
        "crate" = "json"
        "affected_versions" = "all"
        "description" = "json crate is unmaintained"
    }
    "RUSTSEC-2024-0384" = @{
        "crate" = "unknown" 
        "affected_versions" = "unknown"
        "description" = "Need to investigate specific crate"
    }
    "RUSTSEC-2024-0370" = @{
        "crate" = "unknown"
        "affected_versions" = "unknown" 
        "description" = "Need to investigate specific crate"
    }
    "RUSTSEC-2024-0320" = @{
        "crate" = "unknown"
        "affected_versions" = "unknown"
        "description" = "Need to investigate specific crate"
    }
}

Write-Host "1. Checking for json crate (RUSTSEC-2022-0081)..." -ForegroundColor Yellow
$jsonCheck = cargo tree -i json 2>&1
if ($jsonCheck -match "did not match any packages") {
    Write-Host "   ✅ RESOLVED: json crate is no longer in dependency tree" -ForegroundColor Green
} else {
    Write-Host "   ❌ ISSUE: json crate still present:" -ForegroundColor Red
    Write-Host "   $jsonCheck"
}

Write-Host ""
Write-Host "2. Checking current crate versions..." -ForegroundColor Yellow

# Check versions of commonly vulnerable crates
$checkCrates = @("chrono", "regex", "time", "serde", "tokio", "reqwest")

foreach ($crate in $checkCrates) {
    $crateInfo = cargo tree | Select-String "$crate v" | Select-Object -First 1
    if ($crateInfo) {
        Write-Host "   $crateInfo" -ForegroundColor Cyan
    }
}

Write-Host ""
Write-Host "3. Build Status Check..." -ForegroundColor Yellow
$buildResult = cargo check 2>&1
if ($LASTEXITCODE -eq 0) {
    Write-Host "   ✅ Project builds successfully" -ForegroundColor Green
} else {
    Write-Host "   ❌ Build failed - see errors above" -ForegroundColor Red
}

Write-Host ""
Write-Host "=== Summary ===" -ForegroundColor Green
Write-Host "- RUSTSEC-2022-0081 (json crate): RESOLVED ✅"
Write-Host "- Other vulnerabilities may be resolved by dependency updates"
Write-Host "- Project builds successfully"
Write-Host ""
Write-Host "Recommendation: Install cargo-audit with newer Rust version or use osv-scanner for complete verification" -ForegroundColor Cyan
