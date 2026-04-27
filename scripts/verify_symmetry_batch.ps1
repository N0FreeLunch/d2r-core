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
$d2sFiles = Get-ChildItem -Path $InputDir -Filter "*.d2s" -Recurse
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
    Write-Host "[$totalFiles/$($d2sFiles.Count)] Processing: $relativeName" -NoNewline

    # Run SymmetryBitDiff
    $process = Start-Process -FilePath "cargo" -ArgumentList "run", "--bin", "SymmetryBitDiff", "--quiet", "--", "`"$($file.FullName)`"", "--roundtrip", "--json" -NoNewWindow -PassThru -RedirectStandardOutput "tmp_stdout.txt" -RedirectStandardError "tmp_stderr.txt"
    $process.WaitForExit()
    
    $stdout = Get-Content "tmp_stdout.txt" -Raw -ErrorAction SilentlyContinue
    $stderr = Get-Content "tmp_stderr.txt" -Raw -ErrorAction SilentlyContinue

    $fileResult = @{
        file = $relativeName
        success = $false
        error = $null
    }

    if ($process.ExitCode -ne 0) {
        Write-Host " - FAILED (ExitCode: $($process.ExitCode))" -ForegroundColor Red
        $fileResult.error = "ExitCode: $($process.ExitCode). Stderr: $stderr"
        $failedFiles++
    } else {
        try {
            # Find the JSON block (starting with {"success" and ending with })
            # We use (?s) for dot-matches-newline
            if ($stdout -match '(?s)(\{\s*"success".*\})') {
                $cleanJson = $Matches[1]
                $report = $cleanJson | ConvertFrom-Json
                $fileResult.success = $report.success
                
                if (-not $report.success) {
                    Write-Host " - MISMATCH" -ForegroundColor Yellow
                    $failedFiles++
                    
                    # Recursive helper to flatten mismatches
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
                # No success JSON found, check if there's a Rust error/panic
                if ($stdout -match '\{ error: (.*) \}') {
                    $rustError = $Matches[1]
                    Write-Host " - TOOL ERROR ($rustError)" -ForegroundColor Red
                    $fileResult.error = "Tool Error: $rustError"
                } else {
                    Write-Host " - UNKNOWN OUTPUT" -ForegroundColor Red
                    $fileResult.error = "No JSON report found. Raw output: $stdout"
                }
                $failedFiles++
            }
        } catch {
            Write-Host " - JSON PARSE ERROR ($($_.Exception.Message))" -ForegroundColor Red
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

# Ensure output directory exists
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

# Cleanup temp files
Remove-Item "tmp_stdout.txt" -ErrorAction SilentlyContinue
Remove-Item "tmp_stderr.txt" -ErrorAction SilentlyContinue
