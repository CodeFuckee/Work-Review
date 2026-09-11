# E2E：安装 Yue-Review 时不得影响运行中的 Work-Review（两套软件完全独立）
#
# 用例（对应已确认用例表）：
#   E2E-001 旧版运行中静默安装 Yue-Review，旧 Work_Review.exe 进程不被杀
#   E2E-002 安装产物存在（Yue_Review.exe），旧进程仍存活
#   E2E-003 启动新装应用，新旧两进程并存
#   E2E-004 静默卸载 Yue-Review，旧进程不受影响
#
# 用法：
#   powershell -ExecutionPolicy Bypass -File run.ps1 -InstallerPath <Yue-Review_x64-setup.exe 路径>
#
# 断言的是目标状态（旧进程必须存活）。对含 bug 的旧安装包（v1.1.32 及以前）
# 运行时 E2E-001 会失败——那正是要复现的产品缺陷。
param(
  [Parameter(Mandatory = $true)][string]$InstallerPath,
  [string]$ReportDir = ''
)

$ErrorActionPreference = 'Stop'
if (-not $ReportDir) { $ReportDir = Join-Path $PSScriptRoot 'reports' }
$OldBinaryName = 'Work_Review'          # 旧版二进制名（v1.1.32 及以前）
$NewBinaryName = 'Yue_Review'           # 新版二进制名（修复后）
$NewProductName = 'Yue-Review'
$NewInstallDir = Join-Path $env:LOCALAPPDATA $NewProductName
$NewExePath = Join-Path $NewInstallDir "$NewBinaryName.exe"
$OldDataDir = Join-Path $env:APPDATA 'work-review'
$NewDataDir = Join-Path $env:APPDATA 'yue-review'
$TmpDir = Join-Path $env:TEMP 'yue-e2e'

if (-not (Test-Path $InstallerPath)) { throw "安装包不存在: $InstallerPath" }
New-Item -ItemType Directory -Force -Path $ReportDir | Out-Null
New-Item -ItemType Directory -Force -Path $TmpDir | Out-Null

$results = [System.Collections.Generic.List[object]]::new()
function Record-Case([string]$Id, [string]$Name, [bool]$Passed, [string]$Detail) {
  $results.Add([pscustomobject]@{ Id = $Id; Name = $Name; Passed = $Passed; Detail = $Detail })
  $tag = if ($Passed) { 'PASS' } else { 'FAIL' }
  Write-Host "[$tag] $Id $Name — $Detail"
}

function Get-OldPids {
  return @(Get-Process -Name $OldBinaryName -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Id)
}

function Wait-InstallerExit([string]$Exe, [string[]]$ArgList, [int]$TimeoutMs) {
  $p = Start-Process -FilePath $Exe -ArgumentList $ArgList -PassThru
  if (-not $p.WaitForExit($TimeoutMs)) {
    try { $p.Kill() } catch {}
    throw "进程超时未退出: $Exe $($Args -join ' ')"
  }
  return $p.ExitCode
}

# ── 前置：确保有一个旧版 Work_Review.exe 进程在运行 ─────────────────────
$oldExePath = $null
$oldStartedByTest = $false
$oldProcs = @(Get-Process -Name $OldBinaryName -ErrorAction SilentlyContinue)
if ($oldProcs.Count -eq 0) {
  $candidates = @(
    (Join-Path $env:LOCALAPPDATA 'Work Review\Work_Review.exe'),
    (Join-Path $TmpDir 'Work_Review\Work_Review.exe')
  ) | Where-Object { Test-Path $_ }
  if ($candidates.Count -eq 0) {
    Write-Host '本地无旧版 exe，从 Release 下载便携版作为旧版进程载体...'
    $zip = Join-Path $TmpDir 'Work_Review_portable_x64.zip'
    curl.exe -sSL -o $zip 'https://home.chenkaidi.top:509/chenkaidi/work-review/-/releases/v1.1.30/downloads/Work_Review_portable_x64.zip'
    Expand-Archive -Force -Path $zip -DestinationPath (Join-Path $TmpDir 'Work_Review')
    $candidates = @((Join-Path $TmpDir 'Work_Review\Work_Review.exe') | Where-Object { Test-Path $_ })
  }
  if ($candidates.Count -eq 0) { throw '找不到可用的旧版 Work_Review.exe' }
  $oldExePath = $candidates[0]
  Start-Process -FilePath $oldExePath | Out-Null
  $oldStartedByTest = $true
  Start-Sleep -Seconds 4
} else {
  $oldExePath = $oldProcs[0].Path
}

$oldPidsBefore = Get-OldPids
if ($oldPidsBefore.Count -eq 0) { throw '前置失败：未能启动旧版 Work_Review.exe 进程' }
$newDataDirExistedBefore = Test-Path $NewDataDir
Write-Host "前置就绪：旧进程 PID = $($oldPidsBefore -join ', ')（exe: $oldExePath）"
Write-Host "安装包: $InstallerPath"

# ── E2E-001：静默安装，旧进程不被杀 ────────────────────────────────────
$e001Detail = ''
$e001Pass = $false
try {
  $exitCode = Wait-InstallerExit (Resolve-Path $InstallerPath).Path @('/S') 300000
  Start-Sleep -Seconds 3
  $aliveNow = @(Get-OldPids)
  $dead = @($oldPidsBefore | Where-Object { $aliveNow -notcontains $_ })
  $e001Pass = ($dead.Count -eq 0)
  $e001Detail = "安装器退出码 $exitCode；旧进程死亡: [$($dead -join ', ')]，存活: [$($aliveNow -join ', ')]"
} catch {
  $e001Detail = "安装器执行异常: $_"
}
Record-Case 'E2E-001' '旧版运行中安装 Yue-Review，旧进程不被杀' $e001Pass $e001Detail

# ── E2E-002：安装产物存在（新二进制名），旧进程仍存活 ──────────────────
$e002Detail = "新装 exe 不存在: $NewExePath"
$e002Pass = $false
if (Test-Path $NewExePath) {
  $aliveNow = @(Get-OldPids)
  $e002Pass = ($aliveNow.Count -gt 0)
  $e002Detail = "新装 exe 存在: $NewExePath；旧进程存活: [$($aliveNow -join ', ')]"
}
Record-Case 'E2E-002' '安装产物存在且旧进程仍存活' $e002Pass $e002Detail

# ── E2E-003：启动新装应用，新旧两进程并存 ─────────────────────────────
$e003Detail = '前置失败：新装 exe 不存在'
$e003Pass = $false
$newProcStarted = $false
if (Test-Path $NewExePath) {
  Start-Process -FilePath $NewExePath | Out-Null
  $newProcStarted = $true
  Start-Sleep -Seconds 6
  $newAlive = @(Get-Process -Name $NewBinaryName -ErrorAction SilentlyContinue)
  $aliveNow = @(Get-OldPids)
  $e003Pass = ($newAlive.Count -gt 0 -and $aliveNow.Count -gt 0)
  $newPidText = if ($newAlive.Count -gt 0) { ($newAlive | Select-Object -ExpandProperty Id) -join ', ' } else { '' }
  $e003Detail = "新进程 PID: [$newPidText]，旧进程 PID: [$($aliveNow -join ', ')]"
}
Record-Case 'E2E-003' '启动新装应用，新旧两进程并存' $e003Pass $e003Detail

# ── E2E-004：静默卸载不影响旧版 ───────────────────────────────────────
$e004Detail = ''
$e004Pass = $false
try {
  if ($newProcStarted) {
    Get-Process -Name $NewBinaryName -ErrorAction SilentlyContinue | Stop-Process -Force
    Start-Sleep -Seconds 2
  }
  $uninstaller = Join-Path $NewInstallDir 'uninstall.exe'
  if (-not (Test-Path $uninstaller)) { throw "卸载器不存在: $uninstaller" }
  Wait-InstallerExit $uninstaller @('/S') 180000 | Out-Null
  Start-Sleep -Seconds 8  # NSIS 卸载器会派生临时副本自行运行，等待其完成
  $aliveNow = @(Get-OldPids)
  $e004Pass = ($aliveNow.Count -gt 0)
  $e004Detail = "卸载后旧进程存活: [$($aliveNow -join ', ')]；安装目录残留: $(Test-Path $NewInstallDir)"
} catch {
  $e004Detail = "卸载异常: $_"
}
Record-Case 'E2E-004' '静默卸载不影响旧版进程' $e004Pass $e004Detail

# ── 清理与恢复 ────────────────────────────────────────────────────────
if ((Get-OldPids).Count -eq 0 -and $oldExePath -and (Test-Path $oldExePath)) {
  Start-Process -FilePath $oldExePath | Out-Null
  Write-Host "恢复：旧版 Work_Review.exe 已重新启动（测试中被杀）"
}
if (-not $newDataDirExistedBefore -and (Test-Path $NewDataDir)) {
  Remove-Item -Recurse -Force $NewDataDir -ErrorAction SilentlyContinue
  Write-Host "清理：已删除测试产生的 $NewDataDir"
}

# ── 报告 ──────────────────────────────────────────────────────────────
$stamp = Get-Date -Format 'yyyyMMdd-HHmmss'
$passCount = @($results | Where-Object { $_.Passed }).Count
$report = [pscustomobject]@{
  Time = (Get-Date).ToString('yyyy-MM-dd HH:mm:ss')
  Installer = $InstallerPath
  OldPidsBefore = ($oldPidsBefore -join ',')
  Total = $results.Count
  Passed = $passCount
  Failed = $results.Count - $passCount
  Cases = $results
}
$report | ConvertTo-Json -Depth 3 | Set-Content -Encoding UTF8 (Join-Path $ReportDir "installer-isolation-$stamp.json")
Write-Host "`n===== 结果: $($passCount)/$($results.Count) 通过 ====="
if ($passCount -lt $results.Count) { exit 1 }
exit 0
