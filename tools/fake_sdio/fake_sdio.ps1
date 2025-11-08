param(
    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]]$Args
)

function Normalize-Path {
    param(
        [string]$Value,
        [string]$BaseDir
    )

    if (-not $Value) {
        return $null
    }

    $trimmed = $Value.Trim()
    if ($trimmed.StartsWith('"') -and $trimmed.EndsWith('"')) {
        $trimmed = $trimmed.Substring(1, $trimmed.Length - 2)
    }

    if ($trimmed.Length -eq 0) {
        return $null
    }

    if ([System.IO.Path]::IsPathRooted($trimmed)) {
        return $trimmed
    }

    return [System.IO.Path]::GetFullPath((Join-Path -Path $BaseDir -ChildPath $trimmed))
}

$scriptPath = $null
for ($i = 0; $i -lt $Args.Length; $i++) {
    $arg = $Args[$i]
    if ($arg -like "-script:*") {
        $scriptPath = $arg.Split(":", 2)[1]
        break
    }
    if ($arg -eq "-script" -and ($i + 1) -lt $Args.Length) {
        $scriptPath = $Args[$i + 1]
        break
    }
}

if (-not $scriptPath) {
    Write-Host "[Fake SDIO] Missing -script argument"
    exit 1
}

$scriptPath = [System.IO.Path]::GetFullPath($scriptPath)
if (-not (Test-Path -LiteralPath $scriptPath)) {
    Write-Host "[Fake SDIO] Script not found: $scriptPath"
    exit 1
}

$scriptDir = Split-Path -Parent $scriptPath
$logEntries = [System.Collections.Generic.List[string]]::new()
$logEntries.Add("Fake SDIO invoked at $(Get-Date -Format o)")
$logEntries.Add("Args: " + ($Args -join ' '))

$logDir = Join-Path -Path $scriptDir -ChildPath "logs"
$extractDir = $scriptDir

$lines = Get-Content -LiteralPath $scriptPath
foreach ($line in $lines) {
    $trim = $line.Trim()
    if ([string]::IsNullOrWhiteSpace($trim)) {
        continue
    }

    if ($trim.StartsWith('#') -or $trim.StartsWith(';') -or $trim.StartsWith(':')) {
        continue
    }

    if ($trim.StartsWith("echo ")) {
        $message = $trim.Substring(5)
        Write-Host $message
        $logEntries.Add("ECHO: $message")
        continue
    }

    $parts = $trim -split '\s+', 2
    $command = $parts[0].ToLowerInvariant()
    $rest = if ($parts.Length -gt 1) { $parts[1].Trim() } else { "" }

    switch ($command) {
        "extractdir" {
            $path = Normalize-Path -Value $rest -BaseDir $scriptDir
            if ($path) {
                New-Item -ItemType Directory -Force -Path $path | Out-Null
                $extractDir = $path
                $logEntries.Add("extractdir -> $path")
            }
        }
        "logdir" {
            $path = Normalize-Path -Value $rest -BaseDir $scriptDir
            if ($path) {
                New-Item -ItemType Directory -Force -Path $path | Out-Null
                $logDir = $path
                $logEntries.Add("logdir -> $path")
            }
        }
        "writedevicelist" {
            $target = Normalize-Path -Value $rest -BaseDir $scriptDir
            if ($target) {
                $parent = Split-Path -Parent $target
                if ($parent) {
                    New-Item -ItemType Directory -Force -Path $parent | Out-Null
                }
                $content = "[Device]`nName: Fake Device`nSelected: 0`n"
                Set-Content -LiteralPath $target -Value $content -Encoding UTF8
                $logEntries.Add("writedevicelist -> $target")
            }
        }
        "snapshot" {
            $target = Normalize-Path -Value $rest -BaseDir $scriptDir
            if (-not $target) {
                $target = Join-Path -Path $scriptDir -ChildPath "snapshot.txt"
            }
            $parent = Split-Path -Parent $target
            if ($parent) {
                New-Item -ItemType Directory -Force -Path $parent | Out-Null
            }
            Set-Content -LiteralPath $target -Value "snapshot" -Encoding UTF8
            $logEntries.Add("snapshot -> $target")
        }
        default {
            $logEntries.Add("command -> $command $rest")
        }
    }
}

if ($logDir) {
    $logEntries.Add("Completed successfully.")
    $logFile = Join-Path -Path $logDir -ChildPath "fake_sdio.log"
    Set-Content -LiteralPath $logFile -Value $logEntries -Encoding UTF8
}

exit 0
