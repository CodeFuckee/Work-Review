# windows-toolchain.ps1 - toolchain bootstrap for the GitLab CI Windows
# runner (PowerShell executor), invoked from .gitlab-ci.yml
# (.powershell-toolchain template).
#
# Background: the gitlab-runner service (e.g. LocalSystem) usually runs with
# a PATH that lacks the Rust and Node toolchains, so this script locates
# rustup/cargo and node explicitly and prepends them to $env:Path.
#
# Historical defect (introduced by commit 703624a): the node fallback only
# checked C:\Program Files\nodejs, so jobs failed with "node not found" when
# node was installed via nvm-windows / scoop / chocolatey - or not installed
# at all. This script probes all of those layouts and, when nothing is found,
# reports exactly which locations were searched.
#
# NOTE: keep this file ASCII-only. Windows PowerShell 5.1 decodes BOM-less
# UTF-8 files using the system ANSI codepage, so any non-ASCII byte breaks
# parsing on that host.
#
# CI usage:
#   & "$env:CI_PROJECT_DIR\scripts\windows-toolchain.ps1"
#
# Test usage (injectable):
#   -NodeOnly                     skip the rustup bootstrap
#   -NodeCandidateDirs <dirs>     override the default node candidate list
#   -RustupCandidateDirs <dirs>   override the default rustup candidate list
# On success the script prints NODE_DIR=... / RUSTUP_DIR=... for assertions.

[CmdletBinding()]
param(
    [switch]$NodeOnly,
    [string[]]$NodeCandidateDirs = @(),
    [string[]]$RustupCandidateDirs = @()
)

$ErrorActionPreference = "Stop"

# Probe log filled by Find-NodeDir; dumped before throwing so a failed job
# shows exactly which candidate paths were (not) visible. Also record the
# PowerShell running this script, since the Windows runner may invoke a
# different (MSIX-packaged) pwsh than local PowerShell.
$script:NodeProbeLog = [System.Collections.Generic.List[string]]::new()
$script:NodeProbeLog.Add("PSVersion=$($PSVersionTable.PSVersion)")
try { $script:NodeProbeLog.Add("PSProcess=$( (Get-Process -Id $PID).Path )") } catch { }

function Find-RustupDir {
    param([string[]]$CandidateDirs)

    if ($CandidateDirs.Count -gt 0) {
        foreach ($dir in $CandidateDirs) {
            if (Test-Path "$dir\rustup.exe") { return $dir }
        }
        return $null
    }

    # Default: the current account's .cargo\bin, then scan every account
    # under C:\Users for .cargo\bin containing rustup.exe.
    $cargoBin = Join-Path $env:USERPROFILE ".cargo\bin"
    if (Test-Path "$cargoBin\rustup.exe") { return $cargoBin }
    return (
        Get-ChildItem -Path "C:\Users" -Directory -ErrorAction SilentlyContinue |
            ForEach-Object { Join-Path $_.FullName ".cargo\bin" } |
            Where-Object { Test-Path "$_\rustup.exe" } |
            Select-Object -First 1
    )
}

function Get-NvmNodeDir {
    # Scan %APPDATA%\nvm (current account) and
    # C:\Users\<user>\AppData\Roaming\nvm (other accounts); return the
    # directory of the newest v<major>.<minor>.<patch> version containing
    # node.exe.
    $roots = [System.Collections.Generic.List[string]]::new()
    if ($env:APPDATA) { $roots.Add("$env:APPDATA\nvm") }
    Get-ChildItem -Path "C:\Users" -Directory -ErrorAction SilentlyContinue |
        ForEach-Object { Join-Path (Join-Path $_.FullName "AppData\Roaming") "nvm" } |
        ForEach-Object { if (-not $roots.Contains($_)) { $roots.Add($_) } }

    $best = $null
    foreach ($root in $roots) {
        if (-not (Test-Path $root)) { continue }
        foreach ($vd in (Get-ChildItem -Path $root -Directory -ErrorAction SilentlyContinue)) {
            if (-not (Test-Path "$($vd.FullName)\node.exe")) { continue }
            if ($vd.Name -match '^v?(\d+)\.(\d+)\.(\d+)') {
                $ver = [version]::new([int]$Matches[1], [int]$Matches[2], [int]$Matches[3])
                if (-not $best -or $ver -gt $best.Version) {
                    $best = [pscustomobject]@{ Dir = $vd.FullName; Version = $ver }
                }
            }
        }
    }
    if ($best) { return $best.Dir }
    return $null
}

function Find-NodeDir {
    param([string[]]$CandidateDirs)

    if ($CandidateDirs.Count -gt 0) {
        foreach ($dir in $CandidateDirs) {
            if (Test-Path "$dir\node.exe") { return $dir }
        }
        return $null
    }

    # Default candidates, in priority order:
    # 1) official installer directories (nodejs.org; also the default
    #    nvm-windows symlink target)
    # 2) newest nvm-windows version directory
    # 3) scoop shim directories
    # 4) chocolatey shim directory
    $dirs = [System.Collections.Generic.List[string]]::new()
    $dirs.Add("C:\Program Files\nodejs")
    $dirs.Add("C:\Program Files (x86)\nodejs")

    $nvmDir = Get-NvmNodeDir
    if ($nvmDir) { $dirs.Add($nvmDir) }

    Get-ChildItem -Path "C:\Users" -Directory -ErrorAction SilentlyContinue |
        ForEach-Object { Join-Path $_.FullName "scoop\apps\nodejs\current" } |
        Where-Object { Test-Path "$_\node.exe" } |
        ForEach-Object { $dirs.Add($_) }

    if (Test-Path "C:\ProgramData\chocolatey\bin\node.exe") {
        $dirs.Add("C:\ProgramData\chocolatey\bin")
    }

    # De-duplicate while preserving order; return the first hit. Every probed
    # path is logged (PROBE_NODE>) so a failed job shows why node was missed.
    $seen = [System.Collections.Generic.HashSet[string]]::new()
    foreach ($dir in $dirs) {
        if ($seen.Add($dir)) {
            $probe = "$dir\node.exe"
            $psProbe = Test-Path $probe
            $dotnetProbe = [System.IO.File]::Exists($probe)
            $script:NodeProbeLog.Add("$probe => Test-Path=$psProbe .NET=$dotnetProbe")
            if ($dotnetProbe) { return $dir }
        }
    }
    return $null
}

# ---- rustup bootstrap (skipped with -NodeOnly) ----
if (-not $NodeOnly) {
    $rustDir = Find-RustupDir -CandidateDirs $RustupCandidateDirs
    if ($rustDir) {
        $env:Path = "$rustDir;$env:Path"
        Write-Output "RUSTUP_DIR=$rustDir"
    }
    if (-not (Get-Command rustup -ErrorAction SilentlyContinue)) {
        throw "rustup not found. Searched: $env:USERPROFILE\.cargo\bin and every C:\Users\<user>\.cargo\bin. Install Rust via https://rustup.rs so that .cargo\bin is reachable by the gitlab-runner service account."
    }
}

# ---- node bootstrap ----
$nodeDir = Find-NodeDir -CandidateDirs $NodeCandidateDirs
if ($nodeDir) {
    $env:Path = "$nodeDir;$env:Path"
    Write-Output "NODE_DIR=$nodeDir"
}
if (-not (Get-Command node -ErrorAction SilentlyContinue)) {
    if ($NodeCandidateDirs.Count -gt 0) {
        $searched = ($NodeCandidateDirs -join "; ")
    } else {
        $searched = "PATH; C:\Program Files\nodejs; C:\Program Files (x86)\nodejs; nvm-windows version dirs under %APPDATA%\nvm; scoop dirs C:\Users\<user>\scoop\apps\nodejs\current; C:\ProgramData\chocolatey\bin"
    }
    foreach ($probeLine in $script:NodeProbeLog) { Write-Output "PROBE_NODE> $probeLine" }
    Write-Output "PROBE_NODE> USERPROFILE=$env:USERPROFILE"
    Write-Output "PROBE_NODE> PATH=$env:Path"
    throw "node not found. Searched for node.exe in: $searched. Install Node.js 22+ (https://nodejs.org) on the gitlab-runner machine, or via nvm-windows/scoop/chocolatey into a default location."
}