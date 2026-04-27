param (
    [Parameter(Mandatory=$true)]
    [string]$InputJson,
    
    [Parameter(Mandatory=$true)]
    [string]$OutputMd
)

$OutputEncoding = [Console]::InputEncoding = [Console]::OutputEncoding = [System.Text.Encoding]::UTF8;

if (-not (Test-Path $InputJson)) {
    Write-Error "Input JSON not found: $InputJson"
    exit 1
}

$summary = Get-Content $InputJson -Raw | ConvertFrom-Json

$totalFiles = $summary.total_files
$failedFiles = $summary.failed_files
$mismatchRows = $summary.mismatch_rows

$report = @()
$report += "# Symmetry Forensic Report"
$report += ""
$report += "## Summary"
$report += ""
$report += "- **Total Files Processed:** $totalFiles"
$report += "- **Failed/Mismatch Files:** $failedFiles"
$report += "- **Total Mismatch Rows:** $($mismatchRows.Count)"
$report += ""

if ($mismatchRows.Count -eq 0) {
    $report += "### ✅ ALL CLEAR"
    $report += ""
    $report += "No symmetry mismatches were detected in the processed batch."
} else {
    # Aggregate by Segment
    $report += "## Top Mismatch Segments"
    $report += ""
    $report += "| Segment | Count |"
    $report += "| :--- | :--- |"
    
    $segmentCounts = $mismatchRows | Group-Object -Property segment | Sort-Object Count -Descending
    foreach ($group in $segmentCounts) {
        $label = if ($null -eq $group.Name -or $group.Name -eq "") { "Unknown Segment" } else { $group.Name }
        $report += "| $label | $($group.Count) |"
    }
    $report += ""

    # Aggregate by Mismatch Type
    $report += "## Mismatch Types"
    $report += ""
    $report += "| Type | Count |"
    $report += "| :--- | :--- |"
    
    $typeCounts = $mismatchRows | Group-Object -Property mismatch_type | Sort-Object Count -Descending
    foreach ($group in $typeCounts) {
        $label = if ($null -eq $group.Name -or $group.Name -eq "") { "Unknown Type" } else { $group.Name }
        $report += "| $label | $($group.Count) |"
    }
    $report += ""

    # Detailed Table
    $report += "## Detailed Mismatches"
    $report += ""
    $report += "| File | Item Label | Code | Segment | Offset | Type |"
    $report += "| :--- | :--- | :--- | :--- | :--- | :--- |"
    
    foreach ($row in $mismatchRows) {
        $file = $row.file
        $label = $row.item_label
        $code = $row.code
        $segment = if ($null -eq $row.segment -or $row.segment -eq "") { "-" } else { $row.segment }
        $offset = if ($null -eq $row.first_mismatch_offset) { "-" } else { $row.first_mismatch_offset }
        $type = if ($null -eq $row.mismatch_type -or $row.mismatch_type -eq "") { "-" } else { $row.mismatch_type }
        
        $report += "| $file | $label | ``$code`` | $segment | $offset | $type |"
    }
    $report += ""

    $report += "## Actionable Clues"
    $report += ""
    $report += "1. **Content Mismatches** in specific segments (e.g., `Stats`) usually indicate a field size or mapping error."
    $report += "2. **Length Mismatches** often point to missing or extra bits in the bitstream serialization."
    $report += "3. **ChildCount Mismatches** suggest issues with socketed items or nested structures."
}

# Write output
$report | Set-Content -Path $OutputMd -Encoding UTF8

Write-Host "Forensic report generated: $OutputMd"
