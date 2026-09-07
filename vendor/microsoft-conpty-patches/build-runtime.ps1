#requires -Version 7.0
[CmdletBinding()]
param(
    [Parameter(Mandatory)] [string] $BuildRoot,
    [string] $SourceDirectory,
    [string] $OutputFile,
    [string] $WindowsSdkVersion = '10.0.26100.0',
    [switch] $RunTests
)

$ErrorActionPreference = 'Stop'
$revision = '5a830b2bf7c053d5c7ac22208fe5a346cb5dd3dc'
$sourceTag = 'v1.24.11911.0'
$buildRootPath = [IO.Path]::GetFullPath($BuildRoot)
if ($buildRootPath.TrimEnd('\', '/') -eq [IO.Path]::GetPathRoot($buildRootPath).TrimEnd('\', '/')) {
    throw 'BuildRoot must name a dedicated build directory.'
}
New-Item -ItemType Directory -Path $buildRootPath -Force | Out-Null
if (-not $SourceDirectory) { $SourceDirectory = Join-Path $buildRootPath 'upstream' }
if (-not $OutputFile) { $OutputFile = Join-Path $buildRootPath 'OpenConsole.exe' }
$source = [IO.Path]::GetFullPath($SourceDirectory)
$output = [IO.Path]::GetFullPath($OutputFile)
$patch = Join-Path $PSScriptRoot 'conpty-performance.patch'
$pathOptions = Join-Path $PSScriptRoot 'portable-paths.targets'

function Invoke-Checked {
    param([string] $Program, [string[]] $Arguments)
    & $Program @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "Command failed ($LASTEXITCODE): $Program"
    }
}

if (-not (Test-Path -LiteralPath $source)) {
    Invoke-Checked git @('-c', 'advice.detachedHead=false', 'clone', '--depth', '1', '--branch', $sourceTag,
        '--single-branch', 'https://github.com/microsoft/terminal.git', $source)
}
$actualRevision = & git -C $source rev-parse HEAD
if ($LASTEXITCODE -ne 0 -or $actualRevision.Trim() -ne $revision) {
    throw 'The source checkout does not match the pinned revision.'
}
& git -C $source diff --cached --quiet
if ($LASTEXITCODE -ne 0) { throw 'The source checkout has staged changes.' }
$changedFiles = @(& git -C $source diff --name-only)
if ($LASTEXITCODE -ne 0) { throw 'Could not inspect the source checkout.' }
if ($changedFiles.Count -eq 0) {
    Invoke-Checked git @('-C', $source, '-c', 'core.whitespace=cr-at-eol', 'apply', '--check', '--cached', $patch)
    Invoke-Checked git @('-C', $source, '-c', 'core.whitespace=cr-at-eol', 'apply', $patch)
} else {
    # Permit an interrupted build to resume only with the exact shipped patch.
    $currentPatch = Join-Path $buildRootPath 'current-source.patch'
    Invoke-Checked git @('-C', $source, '-c', 'core.whitespace=cr-at-eol', '-c', 'diff.algorithm=myers',
        'diff', '--binary', '--full-index', '--no-ext-diff', '--no-renames', "--output=$currentPatch")
    if ((Get-FileHash -LiteralPath $currentPatch).Hash -ne (Get-FileHash -LiteralPath $patch).Hash) {
        throw 'The source checkout has changes other than the shipped patch.'
    }
}

$msbuild = (Get-Command MSBuild.exe -ErrorAction SilentlyContinue).Source
if (-not $msbuild) {
    $vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio/Installer/vswhere.exe'
    if (-not (Test-Path -LiteralPath $vswhere)) { throw 'VS 2022 Build Tools with the x64 C++ tools is required.' }
    $vsRoot = & $vswhere -latest -products '*' -requires Microsoft.Component.MSBuild -property installationPath
    if ($LASTEXITCODE -ne 0 -or -not $vsRoot) { throw 'MSBuild was not found.' }
    $msbuild = Join-Path $vsRoot 'MSBuild/Current/Bin/amd64/MSBuild.exe'
}

$overrides = @{
    VCPKG_DOWNLOADS = (Join-Path $buildRootPath 'downloads')
    VCPKG_DEFAULT_BINARY_CACHE = (Join-Path $buildRootPath 'vcpkg-cache')
    X_VCPKG_REGISTRIES_CACHE = (Join-Path $buildRootPath 'vcpkg-registries')
    NUGET_HTTP_CACHE_PATH = (Join-Path $buildRootPath 'nuget-http-cache')
}
$previousEnvironment = @{}
foreach ($key in $overrides.Keys) {
    $previousEnvironment[$key] = [Environment]::GetEnvironmentVariable($key, 'Process')
    New-Item -ItemType Directory -Path $overrides[$key] -Force | Out-Null
    [Environment]::SetEnvironmentVariable($key, $overrides[$key], 'Process')
}

Push-Location $source
try {
    $nuget = Join-Path $source 'dep/nuget/nuget.exe'
    foreach ($dependency in @(
        @('Microsoft.Windows.ImplementationLibrary', '1.0.240122.1'),
        @('Microsoft.Windows.CppWinRT', '2.0.250303.1')
    )) {
        Invoke-Checked $nuget @('install', $dependency[0], '-Version', $dependency[1], '-OutputDirectory',
            'packages', '-NonInteractive', '-DirectDownload', '-Source', 'https://api.nuget.org/v3/index.json', '-Verbosity', 'quiet')
    }
    if ($RunTests) {
        Invoke-Checked $nuget @('install', 'Microsoft.Taef', '-Version', '10.93.240607003', '-OutputDirectory',
            'packages', '-NonInteractive', '-DirectDownload', '-ConfigFile', (Join-Path $source 'NuGet.config'), '-Verbosity', 'quiet')
    }

    $buildArguments = @("/p:SolutionDir=$source/", '/p:Configuration=Release', '/p:Platform=x64',
        "/p:WindowsTargetPlatformVersion=$WindowsSdkVersion", "/p:ForceImportAfterCppTargets=$pathOptions",
        '/m:4', '/nologo', '/verbosity:minimal')
    Invoke-Checked $msbuild (@('src/host/exe/Host.EXE.vcxproj') + $buildArguments)
    $builtHost = Join-Path $source 'bin/x64/Release/OpenConsole.exe'
    & (Join-Path $PSScriptRoot 'verify-host.ps1') -HostExecutable $builtHost -PrivateRoots @($source, $buildRootPath)

    if ($RunTests) {
        Invoke-Checked $msbuild (@('src/terminal/parser/ut_parser/Parser.UnitTests.vcxproj') + $buildArguments)
        Invoke-Checked $msbuild (@('src/terminal/adapter/ut_adapter/Adapter.UnitTests.vcxproj') + $buildArguments)
        $testRunner = Join-Path $source 'packages/Microsoft.Taef.10.93.240607003/build/Binaries/x64/TE.exe'
        Invoke-Checked $testRunner @('bin/x64/Release/ConParser.Unit.Tests.dll')
        Invoke-Checked $testRunner @('bin/x64/Release/ConAdapter.Unit.Tests.dll', '/name:*ITerm2SetMarkFirstToken*')
    }

    New-Item -ItemType Directory -Path (Split-Path -Parent $output) -Force | Out-Null
    if ($output -ne $builtHost) { Copy-Item -LiteralPath $builtHost -Destination $output -Force }
    Get-FileHash -LiteralPath $output -Algorithm SHA256
} finally {
    Pop-Location
    foreach ($key in $previousEnvironment.Keys) {
        [Environment]::SetEnvironmentVariable($key, $previousEnvironment[$key], 'Process')
    }
}
