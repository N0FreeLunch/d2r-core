param (
    [Parameter(Mandatory=$true)]
    [string]$InputDir,
    
    [Parameter(Mandatory=$false)]
    [string]$OutputJson = "tmp/symmetry_batch_summary.json"
)

$OutputEncoding = [Console]::InputEncoding = [Console]::OutputEncoding = [System.Text.Encoding]::UTF8;

# Ensure InputDir is absolute and exists
if (-not (Test-Path $InputDir)) {
    Write-Error "Input directory does not exist: $InputDir"
    exit 1
}
$InputDir = (Get-Item $InputDir).FullName
Write-Host "Scanning directory: $InputDir"

# Ensure OutputJson is absolute (handle non-existent file)
$OutputJsonPath = if ([System.IO.Path]::IsPathRooted($OutputJson)) { 
    $OutputJson 
} else { 
    Join-Path (Get-Location) $OutputJson 
}

# Find all .d2s files
if (Test-Path $InputDir -PathType Leaf) {
    $d2sFiles = @(Get-Item $InputDir)
} else {
    $d2sFiles = Get-ChildItem -Path $InputDir -Filter "*.d2s" -Recurse
}

if ($d2sFiles.Count -eq 0) {
    Write-Error "No .d2s files found in $InputDir"
    exit 1
}

Write-Host "Found $($d2sFiles.Count) files."

$allResults = @()
$mismatchRows = @()
$totalFiles = 0
$failedFiles = 0

foreach ($file in $d2sFiles) {
    $totalFiles++
    # Use FullName and replace carefully
    $relativeName = $file.FullName
    if ($file.FullName.StartsWith($InputDir)) {
        $relativeName = $file.FullName.Substring($InputDir.Length).TrimStart("\")
    }
    if ([string]::IsNullOrEmpty($relativeName)) { $relativeName = $file.Name }
    
    Write-Host "[$totalFiles/$($d2sFiles.Count)] Processing: $relativeName" -NoNewline

    $fileResult = @{
        file = $relativeName
        success = $false
        error = $null
        failure_family = $null
    }

    # Phase 1: Structural Integrity Audit (d2save_verify)
    # Using direct execution instead of Start-Process for better exit code capture
    & cargo run --bin d2save_verify --quiet -- "$($file.FullName)" > tmp_v_stdout.txt 2> tmp_v_stderr.txt
    
    if ($LASTEXITCODE -ne 0) {
        Write-Host " - CHECKSUM/HEADER FAIL" -ForegroundColor Magenta
        $fileResult.failure_family = "Checksum/Header"
        $vStderr = Get-Content "tmp_v_stderr.txt" -Raw -ErrorAction SilentlyContinue
        $fileResult.error = "d2save_verify failed. Stderr: $vStderr"
        $failedFiles++
        $allResults += $fileResult
        continue
    }

    # Phase 2: Semantic Symmetry Analysis (SymmetryBitDiff)
    & cargo run --bin SymmetryBitDiff --quiet -- "$($file.FullName)" --roundtrip --json > tmp_stdout.txt 2> tmp_stderr.txt
    $exitCode = $LASTEXITCODE
    
    $stdout = Get-Content "tmp_stdout.txt" -Raw -ErrorAction SilentlyContinue
    $stderr = Get-Content "tmp_stderr.txt" -Raw -ErrorAction SilentlyContinue

    $fileResult.baseline_match = $null
    $fileResult.baseline_mismatch_count = 0

    # Phase 3: Baseline Integrity Audit (d2save_baseline_audit)
    # Search for original fixture in standard location
    $originalDir = Join-Path $InputDir "..\original"
    if (-not (Test-Path $originalDir)) {
        $originalDir = Join-Path (Get-Location) "tests/fixtures/savegames/original"
    }
    $originalFile = Join-Path $originalDir $file.Name

    if (Test-Path $originalFile) {
        & cargo run --bin d2save_baseline_audit --quiet -- "$originalFile" "$($file.FullName)" --json > tmp_b_stdout.txt 2> tmp_b_stderr.txt
        $bStdout = Get-Content "tmp_b_stdout.txt" -Raw -ErrorAction SilentlyContinue
        if ($bStdout -match '(?s)(\{\s*"is_match".*\})') {
            $bReport = $Matches[1] | ConvertFrom-Json
            $fileResult.baseline_match = $bReport.is_match
            $fileResult.baseline_mismatch_count = $bReport.mismatch_count
        }
    }

    if ($exitCode -ne 0) {
        Write-Host " - TOOL ERROR (ExitCode: $exitCode)" -ForegroundColor Red
        $fileResult.failure_family = "Tool Error"
        $fileResult.error = "SymmetryBitDiff ExitCode: $exitCode. Stderr: $stderr"
        $failedFiles++
    } else {
        try {
            if ($stdout -match '(?s)(\{\s*"success".*\})') {
                $cleanJson = $Matches[1]
                $report = $cleanJson | ConvertFrom-Json
                $fileResult.success = $report.success
                
                if (-not $report.success) {
                    Write-Host " - ITEM MISMATCH" -ForegroundColor Yellow
                    $fileResult.failure_family = "Item Symmetry"
                    $failedFiles++
                    
                    function Get-Mismatches($items, $parentLabel = "", $fileName = "") {
                        $rows = @()
                        if ($items -isnot [array]) { $items = @($items) }
                        foreach ($item in $items) {
                            $currentLabel = if ($parentLabel) { "$parentLabel -> $($item.label)" } else { $item.label }
                            if (-not $item.is_match) {
                                $row = @{
                                    file = $fileName
                                    item_label = $currentLabel
                                    code = $item.code
                                    mismatch_type = $item.mismatch_type
                                    segment = $item.segment
                                    first_mismatch_offset = $item.first_mismatch_offset
                                }
                                $rows += $row
                            }
                            if ($item.children -and $item.children.Count -gt 0) {
                                $rows += Get-Mismatches $item.children $currentLabel $fileName
                            }
                        }
                        return $rows
                    }

                    $fileMismatches = Get-Mismatches $report.items "" $relativeName
                    $mismatchRows += $fileMismatches
                } else {
                    Write-Host " - OK" -ForegroundColor Green
                }
            } else {
                if ($stdout -match '\{ error: (.*) \}') {
                    $rustError = $Matches[1]
                    Write-Host " - TOOL ERROR ($rustError)" -ForegroundColor Red
                    $fileResult.failure_family = "Tool Error"
                    $fileResult.error = "Tool Error: $rustError"
                } else {
                    Write-Host " - UNKNOWN OUTPUT" -ForegroundColor Red
                    $fileResult.failure_family = "Tool Error"
                    $fileResult.error = "No JSON report found. Raw output: $stdout"
                }
                $failedFiles++
            }
        } catch {
            Write-Host " - JSON PARSE ERROR ($($_.Exception.Message))" -ForegroundColor Red
            $fileResult.failure_family = "Tool Error"
            $fileResult.error = "JSON Parse Failure: $($_.Exception.Message)"
            $failedFiles++
        }
    }
    $allResults += $fileResult
}

$summary = @{
    total_files = $totalFiles
    failed_files = $failedFiles
    mismatch_rows_count = $mismatchRows.Count
    mismatch_rows = $mismatchRows
    results = $allResults
}

$outputDir = [System.IO.Path]::GetDirectoryName($OutputJsonPath)
if (-not (Test-Path $outputDir)) {
    New-Item -ItemType Directory -Path $outputDir -Force | Out-Null
}

$summary | ConvertTo-Json -Depth 10 | Set-Content -Path $OutputJsonPath -Encoding UTF8

Write-Host "`nBatch completed."
Write-Host "Total Files: $totalFiles"
Write-Host "Failed/Mismatch Files: $failedFiles"
Write-Host "Mismatch Rows: $($mismatchRows.Count)"
Write-Host "Summary saved to: $OutputJsonPath"

Remove-Item "tmp_stdout.txt" -ErrorAction SilentlyContinue
Remove-Item "tmp_stderr.txt" -ErrorAction SilentlyContinue
Remove-Item "tmp_v_stdout.txt" -ErrorAction SilentlyContinue
Remove-Item "tmp_v_stderr.txt" -ErrorAction SilentlyContinue
