#requires -Version 7.0
[CmdletBinding()]
param(
    [Parameter(Mandatory)] [string] $HostExecutable,
    [string[]] $PrivateRoots = @()
)

$ErrorActionPreference = 'Stop'
$path = [IO.Path]::GetFullPath($HostExecutable)
$bytes = [IO.File]::ReadAllBytes($path)
$ascii = [Text.Encoding]::ASCII.GetString($bytes)
$wide = [Text.Encoding]::Unicode.GetString($bytes)
$roots = @($PrivateRoots) + @([Environment]::GetFolderPath('UserProfile'))
foreach ($root in $roots | Where-Object { -not [string]::IsNullOrWhiteSpace($_) } | Select-Object -Unique) {
    foreach ($prefix in @($root.Replace('/', '\'), $root.Replace('\', '/'))) {
        if ($ascii.IndexOf($prefix, [StringComparison]::OrdinalIgnoreCase) -ge 0 -or
            $wide.IndexOf($prefix, [StringComparison]::OrdinalIgnoreCase) -ge 0) {
            throw 'The host embeds a build-machine path; refusing to publish it.'
        }
    }
}

Add-Type -AssemblyName System.Reflection.Metadata
$stream = [IO.File]::OpenRead($path)
$pe = [Reflection.PortableExecutable.PEReader]::new($stream)
$pdbPaths = @()
try {
    foreach ($entry in $pe.ReadDebugDirectory()) {
        if ($entry.Type -eq [Reflection.PortableExecutable.DebugDirectoryEntryType]::CodeView) {
            $codeView = $pe.ReadCodeViewDebugDirectoryData($entry)
            if ($codeView.Path -ne 'OpenConsole.pdb') {
                throw 'The host CodeView record must contain only OpenConsole.pdb.'
            }
            $pdbPaths += $codeView.Path
        }
    }
} finally {
    $pe.Dispose()
    $stream.Dispose()
}
if ($pdbPaths.Count -ne 1) {
    throw 'Expected exactly one CodeView record in the host.'
}

[pscustomobject]@{
    Sha256 = (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash
    Size = $bytes.Length
    CodeViewPath = $pdbPaths[0]
    PrivatePathsFound = $false
}
