<#
.SYNOPSIS
Lumo 桌面端内存采样工具（内存治理方案 §2.1）。

.DESCRIPTION
把 Lumo 主进程与其名下的全部 WebView2 辅助进程视为一个"产品进程组"，
按固定间隔采样并写入 CSV；停止时输出整组与分项的 Private Working Set
中位数 / P95 / 峰值。

- 进程归属：以进程树（ParentProcessId）从 Lumo.exe 向下传递归组，并用
  命令行里的 --user-data-dir 交叉核对；启动时打印进程树供人工抽查一次。
- 指标：Private Working Set（主验收）、Private Bytes/Commit、Working Set、
  CPU%、句柄数、线程数。
- 实现走 CIM（Win32_Process + Win32_PerfFormattedData_PerfProc_Process），
  不用 Get-Counter——本地化 Windows 的计数器名（\进程(...)）会让英文名查询失败。
- CPU% 用相邻两次采样的 KernelMode+UserMode 时间差计算，依赖上一次采样。

.EXAMPLE
# 主基线：release 安装包、关 DevTools、固定 1200×720，跑 S3 30 分钟
powershell -ExecutionPolicy Bypass -File scripts\measure-memory.ps1 -Scenario S3 -LibrarySize 30000 -DurationSec 1800

.EXAMPLE
# 单次采样（冒烟）
powershell -ExecutionPolicy Bypass -File scripts\measure-memory.ps1 -Scenario S0 -Once
#>

param(
  [string]$Scenario = 'S0',
  [string]$Notes = '',
  [string]$LibrarySize = 'unspecified',
  [int]$IntervalSec = 2,
  [int]$DurationSec = 0,       # 0 = 一直采样直到 Ctrl+C（finally 里写汇总）
  [switch]$Once,               # 单次采样后立即退出（工具冒烟用）
  [string]$OutDir = ''
)

$ErrorActionPreference = 'Stop'
$RepoRoot = Split-Path -Parent $PSScriptRoot
$Stamp = Get-Date -Format 'yyyyMMdd_HHmmss'
if (-not $OutDir) { $OutDir = Join-Path $RepoRoot 'scripts\mem-data' }
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
$CsvPath = Join-Path $OutDir "${Stamp}_${Scenario}.csv"
$envPath = Join-Path $OutDir "${Stamp}_${Scenario}.env.txt"
$SumPath = Join-Path $OutDir "${Stamp}_${Scenario}.summary.txt"

# ---------------- 环境登记 ----------------
$gitSha = 'unknown'
try { $gitSha = (git -C $RepoRoot rev-parse HEAD).Trim() } catch {}
$os = Get-CimInstance Win32_OperatingSystem
$ramMiB = [math]::Round($os.TotalVisibleMemorySize / 1024)
$dpi = (Get-ItemProperty 'HKCU:\Control Panel\Desktop\WindowMetrics' -ErrorAction SilentlyContinue).AppliedDPI
$scalePct = if ($dpi) { [math]::Round($dpi / 96 * 100) } else { 'unknown' }
$webviewVersion = 'not-running'
$wvProc = Get-Process -Name msedgewebview2 -ErrorAction SilentlyContinue | Select-Object -First 1
if ($wvProc) {
  try { $webviewVersion = $wvProc.Path | ForEach-Object { (Get-Item $_).VersionInfo.ProductVersion } } catch {}
}
@"
scenario      = $Scenario
started_at    = $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss zzz')
git_sha       = $gitSha
os            = $($os.Caption) build $($os.BuildNumber)
ram_visible   = $ramMiB MiB
scale_applied = $scalePct %
webview2      = $webviewVersion
library_size  = $LibrarySize
interval_sec  = $IntervalSec
notes         = $Notes
"@ | Set-Content -Path $EnvPath -Encoding UTF8
Write-Host "环境登记 -> $EnvPath"; Get-Content $EnvPath | ForEach-Object { Write-Host "  $_" }

# ---------------- 进程组归属 ----------------
function Get-ProcessTree {
  Get-CimInstance Win32_Process -Property ProcessId,ParentProcessId,Name,CommandLine,HandleCount,ThreadCount,WorkingSetSize,KernelModeTime,UserModeTime
}

function Get-LumoGroup {
  <# 返回 @{ ById = hashtable pid->proc; RootCount = n } #>
  $all = Get-ProcessTree
  $byId = @{}
  foreach ($p in $all) { $byId[[uint32]$p.ProcessId] = $p }
  $roots = @($byId.Values | Where-Object { $_.Name -ieq 'Lumo.exe' })
  $group = @{}
  $queue = New-Object System.Collections.Queue
  foreach ($r in $roots) { $group[[uint32]$r.ProcessId] = $r; $queue.Enqueue($r.ProcessId) }
  while ($queue.Count -gt 0) {
    $parent = [uint32]$queue.Dequeue()
    foreach ($p in $byId.Values) {
      $pp = [uint32]$p.ParentProcessId
      if (-not $group.ContainsKey($pp) -and $pp -eq $parent) {
        $group[$pp] = $p
        $queue.Enqueue($pp)
      }
    }
  }
  @{ ById = $group; RootCount = $roots.Count }
}

function Get-Category($proc) {
  if ($proc.Name -ieq 'Lumo.exe') { return 'main' }
  $cl = [string]$proc.CommandLine
  if ($cl -match '--type=renderer') { return 'webview-renderer' }
  if ($cl -match '--type=gpu-process') { return 'webview-gpu' }
  if ($cl -match '--type=utility') { return 'webview-utility' }
  if ($cl -match '--type=crashpad') { return 'crashpad' }
  if ($cl -match '--embedded-browser-webview') { return 'webview-browser' }
  if ($proc.Name -ieq 'msedgewebview2.exe') { return 'webview-browser' }
  return 'other'
}

# ---------------- 采样循环 ----------------
$sw = New-Object System.Diagnostics.Stopwatch
$writer = New-Object System.IO.StreamWriter($CsvPath, $false, [System.Text.Encoding]::UTF8)
$writer.WriteLine('timestamp,scenario,pid,process,category,pws_mib,ws_mib,commit_mib,cpu_pct,handles,threads')

$samples = New-Object System.Collections.Generic.List[object]  # @{ t; groupPws; byCat = @{cat=List[pws]} }
$prevCpu = @{}   # pid -> (kernel+user) 100ns
$prevT = $null
$logicalCores = [Environment]::ProcessorCount
$sampleCount = 0

function ToMiB($bytes) { if ($null -eq $bytes) { 0.0 } else { [math]::Round($bytes / 1MB, 2) } }

Write-Host "`n开始采样：场景 $Scenario，间隔 ${IntervalSec}s" -ForegroundColor Cyan
Write-Host "（停止时输出汇总；归属抽查：下方是当前进程组树）"

function Show-TreeSnapshot {
  $g = Get-LumoGroup
  if ($g.RootCount -eq 0) {
    Write-Host '  [未发现 Lumo.exe —— 请先启动应用再挂采样]' -ForegroundColor Yellow
    return
  }
  foreach ($p in $g.ById.Values | Sort-Object ProcessId) {
    Write-Host ("  pid={0,-7} {1,-22} {2}" -f $p.ProcessId, $p.Name, (Get-Category $p))
  }
}
Show-TreeSnapshot

$sw.Start()
try {
  while ($true) {
    $tickStart = Get-Date
    $tree = Get-LumoGroup
    $group = $tree.ById

    if ($group.Count -gt 0) {
      # 一次 CIM 查询取全部进程的格式化性能数据（本地化无关）
      $perf = @{}
      Get-CimInstance Win32_PerfFormattedData_PerfProc_Process -Property IDProcess,WorkingSetPrivate,PrivateBytes |
        ForEach-Object { $perf[[uint32]$_.IDProcess] = $_ }

      $nowCpu = @{}
      $groupPws = 0.0
      $catPws = @{}
      $ts = (Get-Date).ToUniversalTime().ToString('o')

      foreach ($p in $group.Values) {
        $pidKey = [uint32]$p.ProcessId
        $cat = Get-Category $p
        $perfEntry = $perf[$pidKey]
        $pwsMiB = if ($perfEntry -and $null -ne $perfEntry.WorkingSetPrivate) { ToMiB $perfEntry.WorkingSetPrivate } else { 0.0 }
        $wsMiB = ToMiB $p.WorkingSetSize
        $commitMiB = if ($perfEntry -and $null -ne $perfEntry.PrivateBytes) { ToMiB $perfEntry.PrivateBytes } else { 0.0 }

        # CPU%：与上次采样的内核+用户时间差（上次不在组内则本轮回 0）
        $cpuTot = [uint64]$p.KernelModeTime + [uint64]$p.UserModeTime
        $nowCpu[$pidKey] = $cpuTot
        $cpuPct = 0.0
        if ($prevT -and $prevCpu.ContainsKey($pidKey)) {
          $deltaTicks = [double]($cpuTot - $prevCpu[$pidKey])
          $deltaSec = ($tickStart - $prevT).TotalSeconds
          if ($deltaSec -gt 0) {
            # 100ns ticks -> seconds；除以核数与墙钟，得整组口径的单进程 CPU%
            $cpuPct = [math]::Round($deltaTicks / 10000000.0 / $deltaSec / $logicalCores * 100, 2)
          }
        }

        $writer.WriteLine('{0},{1},{2},{3},{4},{5},{6},{7},{8},{9},{10}' -f `
          $ts, $Scenario, $pidKey, $p.Name, $cat, $pwsMiB, $wsMiB, $commitMiB, $cpuPct, $p.HandleCount, $p.ThreadCount)

        $groupPws += $pwsMiB
        if (-not $catPws.ContainsKey($cat)) { $catPws[$cat] = New-Object System.Collections.Generic.List[double] }
        $catPws[$cat].Add($pwsMiB)
      }

      $samples.Add([pscustomobject]@{ T = $tickStart; GroupPws = $groupPws; ByCat = $catPws })
      $prevCpu = $nowCpu
      $prevT = $tickStart
      $sampleCount++
      if ($sampleCount % 15 -eq 0) {
        Write-Host ("  [{0}] 样本 {1}，整组 PWS {2} MiB" -f (Get-Date -Format 'HH:mm:ss'), $sampleCount, $groupPws)
      }
    }
    else {
      $prevT = $null; $prevCpu = @{}
      Write-Host ("  [{0}] 未发现 Lumo 进程组，等待…" -f (Get-Date -Format 'HH:mm:ss')) -ForegroundColor DarkGray
    }

    if ($Once) { break }
    if ($DurationSec -gt 0 -and $sw.Elapsed.TotalSeconds -ge $DurationSec) { break }
    Start-Sleep -Seconds $IntervalSec
  }
}
finally {
  $writer.Flush(); $writer.Close()
}

# ---------------- 汇总 ----------------
function Stats-Of([System.Collections.Generic.List[double]]$v) {
  if (-not $v -or $v.Count -eq 0) { return @{ Median = 0; P95 = 0; Peak = 0; N = 0 } }
  $sorted = $v | Sort-Object
  $median = $sorted[[int][math]::Floor(($sorted.Count - 1) / 2)]
  $p95idx = [int][math]::Ceiling($sorted.Count * 0.95) - 1
  @{ Median = [math]::Round($median, 1); P95 = [math]::Round($sorted[[math]::Max(0, $p95idx)], 1); Peak = [math]::Round($sorted[-1], 1); N = $sorted.Count }
}

$lines = @()
$lines += "scenario=$Scenario samples=$($samples.Count) csv=$CsvPath"
if ($samples.Count -gt 0) {
  $g = Stats-Of ([System.Collections.Generic.List[double]]::new([double[]]($samples | ForEach-Object { $_.GroupPws })))
  $lines += ("整组 Private Working Set  MiB：median={0}  p95={1}  peak={2}" -f $g.Median, $g.P95, $g.Peak)

  $cats = $samples | ForEach-Object { $_.ByCat.Keys } | Sort-Object -Unique
  foreach ($cat in $cats) {
    $vals = [System.Collections.Generic.List[double]]::new()
    foreach ($s in $samples) { if ($s.ByCat.ContainsKey($cat)) { $vals.AddRange($s.ByCat[$cat]) } }
    $st = Stats-Of $vals
    $lines += ("  {0,-18} median={1,8}  p95={2,8}  peak={3,8}" -f $cat, $st.Median, $st.P95, $st.Peak)
  }
}
else {
  $lines += '没有采到任何样本（Lumo 未运行？）'
}
$lines | Set-Content -Path $SumPath -Encoding UTF8
Write-Host "`n汇总 -> $SumPath" -ForegroundColor Cyan
$lines | ForEach-Object { Write-Host $_ }
