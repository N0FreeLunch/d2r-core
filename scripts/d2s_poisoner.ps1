param (
    [Parameter(Mandatory=$true)]
    [string]$InputFile,

    [Parameter(Mandatory=$true)]
    [long]$BitOffset,

    [Parameter(Mandatory=$true)]
    [string]$OutputFile
)

$OutputEncoding = [Console]::InputEncoding = [Console]::OutputEncoding = [System.Text.Encoding]::UTF8

if (-not (Test-Path $InputFile)) {
    Write-Error "Input file not found: $InputFile"
    exit 1
}

$bytes = [System.IO.File]::ReadAllBytes($InputFile)
$byteLen = $bytes.Length

if ($BitOffset -lt 0 -or $BitOffset -ge ($byteLen * 8)) {
    Write-Error "BitOffset $BitOffset is out of bounds (0 to $($byteLen * 8 - 1))"
    exit 1
}

$byteIndex = [Math]::Floor($BitOffset / 8)
$bitIndex = $BitOffset % 8

$oldByte = $bytes[$byteIndex]
# LSB-first assumption: bit 0 is 0x01
$mask = [byte](1 -shl $bitIndex)
$newByte = $oldByte -bxor $mask
$bytes[$byteIndex] = $newByte

# Ensure output directory exists
$outputDir = Split-Path $OutputFile -Parent
if ($outputDir -and -not (Test-Path $outputDir)) {
    New-Item -ItemType Directory -Path $outputDir -Force | Out-Null
}

[System.IO.File]::WriteAllBytes($OutputFile, $bytes)

Write-Host "Mutation Summary:"
Write-Host "  Input: $InputFile"
Write-Host "  Output: $OutputFile"
Write-Host "  BitOffset: $BitOffset"
Write-Host "  ByteIndex: $byteIndex"
Write-Host "  BitIndex: $bitIndex"
Write-Host ("  ByteChange: 0x{0:X2} -> 0x{1:X2}" -f $oldByte, $newByte)
Write-Host "Successfully flipped 1 bit."
