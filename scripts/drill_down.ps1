param (
    [Parameter(Mandatory=$true)]
    [string]$FileA,

    [Parameter(Mandatory=$true)]
    [string]$FileB,

    [Parameter(Mandatory=$true)]
    [long]$BitOffset,

    [Parameter(Mandatory=$false)]
    [int]$WindowSize = 64
)

$OutputEncoding = [Console]::InputEncoding = [Console]::OutputEncoding = [System.Text.Encoding]::UTF8

if (-not (Test-Path $FileA)) { Write-Error "File A not found: $FileA"; exit 1 }
if (-not (Test-Path $FileB)) { Write-Error "File B not found: $FileB"; exit 1 }

$bytesA = [System.IO.File]::ReadAllBytes($FileA)
$bytesB = [System.IO.File]::ReadAllBytes($FileB)

$maxBitA = $bytesA.Length * 8
$maxBitB = $bytesB.Length * 8

$startBit = [Math]::Max(0, $BitOffset - [Math]::Floor($WindowSize / 2))
$endBit = [Math]::Min([Math]::Min($maxBitA, $maxBitB), $startBit + $WindowSize)

Write-Host "Forensic Drill-down (BitWindow $WindowSize)"
Write-Host "  File A: $FileA"
Write-Host "  File B: $FileB"
Write-Host "  Target Bit: $BitOffset"
Write-Host ""

Write-Host " Offset | A | B | Diff"
Write-Host "--------|---|---|------"

for ($i = $startBit; $i -lt $endBit; $i++) {
    $byteIdx = [Math]::Floor($i / 8)
    $bitIdx = $i % 8
    
    $bitA = if ($byteIdx -lt $bytesA.Length) { ($bytesA[$byteIdx] -shr $bitIdx) -band 1 } else { "-" }
    $bitB = if ($byteIdx -lt $bytesB.Length) { ($bytesB[$byteIdx] -shr $bitIdx) -band 1 } else { "-" }
    
    $marker = if ($bitA -ne $bitB) { "  ***" } else { "" }
    $pointer = if ($i -eq $BitOffset) { " <--" } else { "" }
    
    Write-Host ("{0,7} | {1} | {2} |{3}{4}" -f $i, $bitA, $bitB, $marker, $pointer)
}

if ($BitOffset -ge [Math]::Min($maxBitA, $maxBitB)) {
    Write-Host "Warning: Target BitOffset $BitOffset is beyond one of the files." -ForegroundColor Yellow
}
