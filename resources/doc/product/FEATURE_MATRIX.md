# Lumo 功能与平台矩阵（FEATURE_MATRIX）

> 文档状态：Draft v2（I0 / PC-002）
> 事实基线：Lumo v1.8.1，commit `ba15a07`，核对日期 2026-09-18
> 本文件是 Lumo **唯一**的功能状态事实源。README、VISION、Release Notes、官网文案只允许引用本表，不得复制出一份独立维护的状态列表。
> 本文所有条目均以**本人逐行核验代码**为准（v1 稿曾采信二手盘点，其中"系统托盘""Windows 媒体键注册""macOS 毛玻璃""11 种格式白名单"四项不实，已删除）。

## 0. 状态定义与判定规则

| 状态 | 含义 | 判定门槛 |
|---|---|---|
| **Stable** | 可对外承诺 | 代码实现存在 **且** 目标平台有可追溯的真实验证记录 |
| **Beta** | 功能完整可用，但未在目标平台验证或未达规模门槛 | 有实现 + 至少一处已知限制 |
| **Experimental** | 不对外承诺，可能变更或移除 | 默认关闭 / 依赖外部服务 / 有未修复的数据或隐私缺陷 / 仅模拟器验证 |
| **未实现** | 仅设计文档、注释或占位，代码中不存在 | — |

**从严原则**：
1. 代码里存在跨平台分支 ≠ 该平台可用。全仓 `#[cfg(target_os)]` **只有 android 与 not-android 两种二分**（出现在 `commands/scanner.rs`、`lib.rs`、`services/ai.rs`、`services/platform.rs`、`services/secret.rs` 五个文件），**不存在任何 windows / macOS / linux 专属分支**，即桌面三平台共用同一份代码路径、无平台适配层。
2. 底层库能力 ≠ 产品支持。`symphonia` 开了 `all` features（`Cargo.toml:27`）不代表支持对应格式，一律以扫描白名单为准（§3）。
3. 模拟器验证 ≠ 真机验证（§1 Android 行）。

## 1. 平台基线状态

| 平台 | 最新对外版本 | CI 构建 | 签名 | 真实验证证据 | 矩阵结论 |
|---|---|---|---|---|---|
| **Windows x64** | 1.8.1 | 是（`release.yml:16-22` matrix） | ❌ 无 Authenticode | **有**：日常使用 + 迭代记录 + `perf-check*.log` 等本机 SQLite 实测 | **唯一 Beta+ 平台**（缺自动化回归与签名，未达 Stable） |
| **Android aarch64** | 1.2.0 起（仅 aarch64） | 独立 job | ✅ keystore 签名 + `apksigner verify` 列入清单 | **仅 Pixel 7 模拟器（x86_64 / Android 17）**；`MA0_…:168` 明写「真机未接入」，`发布检查清单.md:22-35` 真机冒烟矩阵 **10 项全部未勾选** | **Experimental**（详见 §5 缺口 G-01） |
| **macOS** | 未声明 | 是（universal） | ❌ 无 codesign / notarize | **无任何运行验证记录** | **技术预览** |
| **Linux** | 未声明 | 是（ubuntu-22.04，仅 `apt install` 依赖） | ❌ | **无任何运行验证记录**；`Cargo.toml:60-61` keyring 仅启用 `windows-native`+`apple-native`，**Linux 无钥匙串后端**，凭据走弱加密回退 | **不建议对外发布** |

> ⚠️ **必须向用户澄清的既有失真**：`README.md:132-139` 将 MA0–MA5 全部勾选为完成，但移动端真机验证一项未做。本矩阵以 `发布检查清单.md` 的实际勾选状态为准，README 需在 I0 内修正。

## 2. 能力矩阵

图例：✅ 实现且可用 · ⚠️ 实现但受限/未验证 · ❌ 不存在 · 🖥 仅桌面 · 📱 仅移动端

### 2.1 曲库与浏览

| 能力 | Win | macOS | Linux | Android | 状态 | 证据与限制 |
|---|---|---|---|---|---|---|
| 本地目录扫描 | ✅ | ⚠️ | ⚠️ | ✅ | Beta(Win) / Experimental(其余) | `services/scanner.rs` 并行流水线；进度事件 emit |
| WebDAV 远程扫描 | ✅ | ⚠️ | ⚠️ | ⚠️ | Beta | 走同一白名单 `scanner.rs:303`；服务端差异未验证（I3） |
| 音频格式入库 | 5 种 | 5 种 | 5 种 | 5 种 | Stable | **仅 `mp3 flac wav m4a aac`**（`scanner.rs:19-20`），见 §3 |
| 标签解析（lofty 0.21） | ✅ | ⚠️ | ⚠️ | ✅ | Beta | `services/metadata.rs`，按容器探测 |
| 封面本地提取 | ✅ | ⚠️ | ⚠️ | ✅ | Beta | 提取入 `artwork` 表 + 文件缓存 |
| 在线歌词（LRCLIB） | ✅默认关 | ⚠️ | ⚠️ | ✅默认关 | **Beta** | `commands/library.rs:274`；**无请求超时**；见 `NETWORK_BEHAVIOR.md` §2A |
| 在线封面（网易云+iTunes） | ✅默认关 | ⚠️ | ⚠️ | ✅默认关 | **Beta** | `services/cover.rs:88,110,130`；依赖非公开接口，R-01 |
| 歌曲/文件分离与多音源归并 | ✅ | ⚠️ | ⚠️ | ✅ | Beta | `services/file_priority.rs`（注意其分支含白名单外格式，属死代码） |
| 全局搜索 | ✅ | ⚠️ | ⚠️ | ✅ | Beta | `TopBar.vue:99` / `MobileSearch.vue:38`；**无拼音搜索**（VISION 承诺，未实现） |
| 文件夹视图 | ✅ | ⚠️ | ⚠️ | ⚠️ | Beta | 移动端无 FolderView，来源列表即入口 |
| 批量操作（入队/入歌单） | ✅ | ⚠️ | ⚠️ | ⚠️ | Beta | 移动端缺批量删除与批量改标 |
| 最近播放 / 播放历史 | ✅ | ⚠️ | ⚠️ | ✅ | Beta | `RecentlyPlayed.vue:96-99` **只读，无删除/清空** |

### 2.2 播放

| 能力 | Win | Android | 状态 | 证据与限制 |
|---|---|---|---|---|
| 播放/暂停/Seek/音量 | ✅ | ✅ | Beta | rodio 0.19 → cpal；Android 由 cpal 内部走 oboe，**Rust 侧无后端选择、无 cfg 分支** |
| 队列 + 四种播放模式 | ✅ | ✅ | **Stable 候选** | `services/queue.rs`，全仓唯一有单测的模块（5 个） |
| WebDAV 流播 + Range + 透明缓存 | ✅ | ⚠️ | Beta | 缓存路径 `{app_data_dir}/audio_cache/{id}`（`cache.rs:45,60`，**非硬编码外部存储**）；Android 该目录随应用卸载、私有目录可能空间不足 → 需真机复核 |
| 后台播放（灭屏续播） | — | ⚠️ | **Experimental** | Kotlin 前台服务 `MediaPlaybackService.kt`，**≥30min 灭屏续播仅模拟器观察，无真机证据** |
| 通知栏五键 / 锁屏控制 | — | ⚠️ | **Experimental** | `MediaPlaybackService.kt`（使用平台 `MediaSession`，非 `MediaSessionCompat`）；真机通知栏行为未验证 |
| 音频焦点抢占 / 拔耳机暂停 / 来电暂停 | — | ⚠️ | **Experimental** | 同上，仅模拟器；`BECOMING_NOISY` 真机行为未验证 |
| 系统媒体键（桌面） | ⚠️ | — | **Experimental** | **无全局快捷键插件、无原生注册代码**；仅 WebView 内 `navigator.mediaSession`（`src/stores/player.ts`），是否进入 Windows SMTC **未验证** |
| 键盘快捷键 | ✅ | — | Beta | `App.vue:27-80` 窗口内 `keydown`，非全局 |
| **系统托盘** | ❌ | ❌ | **未实现** | 全仓 `grep tray` 零命中。VISION/规划未列，但常被误认为存在 |
| 沉浸播放页 | ✅ | ✅ | Beta | 桌面 `NowPlayingImmersive.vue` / 移动 `MobileNowPlaying.vue` 各一套 |
| 音频输出设备选择 | ❌ | ❌ | **未实现** | `OutputStream::try_default`，无设备枚举 UI |
| Gapless 无缝切歌 | ⚠️ | ⚠️ | **未验证** | 代码有流式播放，但未做 gapless 专项验证 |

### 2.3 数据、集成与辅助

| 能力 | Win | Android | 状态 | 证据与限制 |
|---|---|---|---|---|
| 歌单（建/删/改/收藏） | ✅ | ⚠️ | Beta | 移动端无重命名入口 |
| 智能歌单 🖥 | ✅ | ❌ | Beta / 移动未实现 | `SmartPlaylistView.vue:412-443`；移动端无对应视图分支 |
| **AI 电台** 🖥 | ✅默认关 | ❌ | Beta（桌面）/ 移动未接线 | 决策 D-01。后端 `services/ai.rs` **无平台 cfg 门控，两端都编译进去**（`keyring_set/get/delete` 在 Android 走 `machine_key_*` 回退）；仅缺移动端 UI 入口。关闭态点「测试连接」仍外发（`commands/ai.rs:67-74`）→ 修复前不升 Stable |
| 数据库备份恢复（WebDAV 快照） | ✅ | ✅ | **Beta** | 整库覆盖、无合并；**4 项 P0 数据风险待修（§5 G-02~G-05）**；桌面恢复后需重启而 `platform::restart_app` 在桌面为空实现（`platform.rs:269`）→ **需人工复核生效路径** |
| 凭据安全存储 | ✅ | ⚠️ | **Beta** | 桌面 keyring（Win/macOS 原生后端）；**Linux 无后端→弱加密回退**；Android 机器绑定派生密钥 XOR（非认证加密）→ I2 |
| 应用内更新检查 📱 | ❌ | ✅手动 | Beta | `MobileSettings.vue:147` fetch api.github.com；**桌面「检查更新」仅跳转 Releases 页**；无 updater 插件 |
| 诊断包导出 📱 | ❌ | ✅ | Beta | `MobileSettings.vue:218-260` share 插件；桌面 `debug.rs` 无导出 |
| 日志文件落盘 | ❌ | ❌ | **未实现** | `tracing_subscriber::fmt::init()` 仅 stdout（`lib.rs:228`）；Android 走 logcat → 「打开日志目录」在多数平台为空目录，I5 处理 |
| 深浅色主题 | ✅ | ✅ | Beta | `stores/ui.ts`；Android 另有系统主题色自适应（`platform.rs`） |
| 移动端本地目录建议 | — | ✅ | Beta | `platform.rs:125-142` 硬编码 9 个候选（含网易云/酷狗/QQ音乐下载目录），仅返回实际存在的目录；属对第三方应用目录结构的隐式依赖 |
| SAF 持久化 URI 授权 | — | ❌ | **未实现** | 全仓无 `takePersistableUriPermission` / `EXTRA_INITIAL_URI`；Android 走「运行时 `READ_EXTERNAL_STORAGE`(≤32) / `READ_MEDIA_AUDIO`(33+) + 手填绝对路径」，非 SAF |
| 本地目录候选（移动） | — | ✅ | Beta | `services/platform.rs:125-142` 硬编码 `/storage/emulated/0` + 9 个候选（含网易云/酷狗/QQ 音乐下载目录），仅返回实际存在者；属对第三方目录布局的隐式依赖 |
| 权限引导与跳转设置 | — | ⚠️ | **Experimental** | `platform.rs:73-83` `startActivity(Settings.ACTION_APPLICATION_DETAILS_SETTINGS)`；仅模拟器验证 |
| 移动端返回键导航 | — | ⚠️ | **Experimental** | `platform.rs:189-199` JNI `lumo_handle_back_key` 栈式路由，`MainActivity.kt:21-25` 先交 JS 再 `finishAffinity()`；仅模拟器验证 |
| 自绘标题栏窗口控制 | ✅ | — | Beta | `decorations: false` + `data-tauri-drag-region`（`TopBar.vue:5`）；`capabilities/default.json` 授予 `core:window:allow-close/minimize/toggle-maximize`；**未实现跨平台最大化还原逻辑** |
| 移动端「即将上线」占位入口 | — | ⚠️ | **已知缺陷** | `MobileContentView.vue:581-594` 智能推荐/排行榜/动态库 → 统一「即将上线」占位；其中智能歌单**桌面已实现**，属功能不可达而非未实现 |

## 3. 已验证格式清单（对外口径，唯一真源）

- **入库白名单（全部平台一致）**：`mp3`、`flac`、`wav`、`m4a`、`aac`
  证据：`src-tauri/src/services/scanner.rs:19-20`，用于本地遍历（`:476`）与 WebDAV（`:303`）。
- **重复定义（技术债）**：同一字面量在 `repositories/track_repo.rs:263` 重复出现，两处必须同步修改，I1 收敛为单一常量。
- **明确不入库**：`ape`、`wv`、`ofr`、`alac`、`aiff`、`caf`、`ac3`、`eac3`、`opus`、`ogg`、`wma`、`aif`、`pcm`、`mkv` —— 无论 symphonia 是否能解码。
- **对外可说**：「支持 MP3、FLAC、WAV、M4A/AAC 五种常见格式」。
- **对外不可说**：任何超出上述五项的格式承诺；VISION §3.1 的「MP3 / FLAC / APE / WAV / OGG / AAC / WMA 等」为**未实现声明**，需在 I0 标注。

## 4. 产品声明红线条款

1. **不使用"同步"描述备份恢复**（决策 D-02）。当前实现为整库快照覆盖，无冲突处理与实体合并；"同步"保留给未来具备合并能力的实现。
2. **不以底层库 features 代替产品支持**，格式一律以 §3 白名单为准。
3. **不使用"原生支持 Linux/macOS""全平台"表述**；macOS/Linux 统一称「技术预览，未经真实验证」。
4. **不使用"20 万曲库不劣化"**。唯一实测为 6000 曲库级别扫描，其余为设计目标；体积/内存声明须待 I3 `PERFORMANCE_BASELINE.md` 实测区间。
5. **AI 电台为可选增强（Beta）**，不得写入核心功能栏（决策 D-01）。
6. **Android 在真机冒烟矩阵完成前不得称"已发布正式版"**；当前对外应表述为「预览版」。
7. 任何对外功能声明必须能在本表找到对应「平台 + 状态 + 验证证据」三元组，否则不予发布。

## 5. 缺口登记（阻塞 Stable 的硬事实）

| ID | 缺口 | 严重度 | 归属 |
|---|---|---|---|
| G-01 | Android **零真机验证**，但 README 宣称 MA0–MA5 完成、CI 已能产出签名 Release APK | **P0（产品承诺失真）** | I0 修文案；真机矩阵在 I6 Beta 前完成 |
| G-02 | 迁移无事务，崩溃后重启触发 `duplicate column` → `init_db` 失败 → `lib.rs:243` `.expect()` panic，**应用无法启动且曲库半迁移** | **P0（数据损坏）** | I3 |
| G-03 | WebDAV XML 解析错误被静默吞掉（`webdav.rs:287` `Err(_) => break`）→ `scan_failed` 仍为 false → **整个来源记录被批量误标 missing** | **P0（数据丢失）** | I3 |
| G-04 | 恢复失败时回滚错误被 `let _ =` 丢弃，随后**无条件删除唯一回退副本**（`commands/sync.rs:117,133-134`） | **P0（不可恢复）** | I3 |
| G-05 | 备份恢复后用户凭据丢失（快照剔除 `credential_ref`/密码，`services/sync.rs:167-172`）；上传无远端 temp+rename，断流即覆盖上一份好快照 | **P0（数据损失）** | I3 |
| G-06 | 隐私开关存 localStorage，可被直连 IPC 绕过（`stores/ui.ts:126` 自称"双保险"） | P1 | I2 |
| G-07 | 扫描无 job id/generation、无取消机制；`ScanGuard` 创建前的 `?` 早退会让来源永久标记为扫描中（`commands/scanner.rs:242-260`） | P1 | I3 |
| G-08 | `last_scan_at` 在失败路径也无条件刷新，无 `last_success_at`；前端一律显示"刚刚扫描"，`last_error` 无任何组件渲染 | P1 | I3 |
| G-09 | 缓存下载 client **无总超时**（`webdav.rs:46-49`）；builder 失败回退 `Client::new()` 为零超时；快照上传被 60s 总超时掐断 | P1 | I3 |
| G-10 | 截断下载被当作有效缓存（rename 前只判 `bytes != 0`，不比对 DB `file_size`；`is_cached` 只看 `len>0`）→ 永久损坏播放 | P1 | I3 |
| G-11 | 缓存淘汰按 mtime 无正在播放保护，`clear()` 会删正在播放文件；`mark_downloading` 非 RAII，panic 后永久卡"正在缓存中" | P1 | I3 |
| G-12 | 429/5xx 无退避；`HttpRangeReader` 3 次重试无 sleep；队列表自动切歌失败时以 250ms 节奏打全队列 | P2 | I3 |
| G-13 | 扫描无拼音搜索、无音频输出设备选择、无系统托盘（VISION 承诺但未实现） | P3 | 范围外，入 Post-Stable Backlog |

## 6. 复核待办

| # | 待验证 | 影响条目 |
|---|---|---|
| V-01 | Android 真机冒烟矩阵 10 项 | §1 Android 行、G-01 |
| V-02 | macOS 核心链路一轮（重点无托盘下的退出闸门、恢复后重启） | 全平台列、§2.3 备份恢复 |
| V-03 | Linux `cargo build` 是否真能通过 keyring 链接 + ALSA 运行时 | §1 Linux 行 |
| V-04 | 白名单 5 格式在双端真实播放（含 m4a/aac 边界） | §3 |
| V-05 | Android `audio_cache` 落在应用私有目录时的空间与卸载行为 | §2.2 流播 |
| V-06 | 桌面恢复后重启生效路径（`restart_app` 桌面空实现） | §2.3 |
| V-07 | WebView `navigator.mediaSession` 是否真的进入 Windows SMTC | §2.2 媒体键 |

## 7. 与既有文档的差异（I0 内需消化）

| 文档 | 失真点 | 处置 |
|---|---|---|
| `README.md:132-139` | MA0–MA5 全勾完成，实际真机零验证 | 改为引用本矩阵，标注预览版 |
| `README.md:11` | 「安装包体积大幅缩小」无实测 | 删具体表述，引 I3 基准 |
| `README.md:32-52` | 目录结构停留在早期（`src/store/`、只列 main/lib.rs） | 重写 |
| `VISION.md:59-61` | 「不发送任何网络请求…没有更新检查」 | 与代码冲突（歌词/封面/AI/GitHub），改由 `PRODUCT_CHARTER.md` 取代 |
| `VISION.md:112-117` | 「跨设备数据同步」「Last-Write-Wins 冲突解决」 | 标注为未实现的未来设计 |
| `VISION.md:267` | 「不做 AI 功能」 | 已由 D-01 取代 |
| `VISION.md:74` | 格式清单含 APE/OGG/WMA | 以 §3 为准 |
| `VISION.md:255` | 「安装包 < 10MB，内存 < 80MB」 | 无实测，待 I3 |
| `Lumo_本地音乐播放器_完整规划与设计.md:149,180` | 「同步冲突处理」「20 万曲库」 | 标注未来设计/目标 |
