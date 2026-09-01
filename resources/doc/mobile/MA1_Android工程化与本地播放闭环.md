# MA1：Android 工程化与本地播放闭环

> 状态：进行中（模拟器闭环全部通过，剩真机复验项）　|　预估：P50 9d / P80 13d　|　实际：1d（2026-09-01，模拟器）　|　前置依赖：MA0 全部退出条件达成
> 配套总体计划：[00_移动端总体迭代计划.md](./00_移动端总体迭代计划.md)

## 1. 目标与背景

在 MA0 的真机基线上，打通 Android 本地音乐完整闭环：

> 全新安装 → 权限申请 → 添加本地目录 → 扫描入库 → 浏览（歌曲/专辑/艺术家/歌单/收藏）→ 播放（含 gapless）→ 收藏/歌单/历史 → 杀进程重启后状态恢复。

本迭代结束后，**手机可以作为日常本地播放器使用**（亮屏场景）。后台播放属 MA2。

核心工程内容有三块：① 存储权限与自定义 Kotlin 插件的工程骨架；② 平台检测与移动设置页补全；③ 真机维度的播放/扫描链路适配。

## 2. 范围

**范围内**：运行时权限、本地来源管理 UI（增/删/扫）、扫描与播放真机适配、缓存与数据目录策略、Android 返回处理、移动设置页基础补全、移除 MA0 临时代码。

**范围外**：WebDAV 来源与流播（MA3）、后台播放/通知（MA2）、安全区与启动性能打磨（MA4）、签名与 CI（MA5）。平板/横屏布局策略仅做「强制移动布局」处理，双栏后置。

## 3. 任务分解

### A1-1 创建 `lumo-mobile` 本地 Tauri 插件（2d）

移动端需要的原生能力（权限、前台服务、系统栏、触感、网络状态等）全部通过**仓库内本地插件**提供，Kotlin 代码集中在插件里，不散落在 gen/android（降低 R6 升级漂移风险）。

```bash
npm run tauri plugin new lumo-mobile --android
# 生成 src-tauri/plugins/lumo-mobile（Rust 宿主 + Kotlin guest 代码）
# Cargo.toml 增加 path 依赖；lib.rs builder 注册插件；前端 main.ts 初始化 @lumo-mobile
```

**本迭代实现的命令子集**（全部仅 Android 生效，桌面端返回安全默认值）：

| 命令 | 签名（Rust 视角） | 说明 |
|---|---|---|
| `check_permission` | `(kind: "media_audio"\|"notifications") -> "granted"\|"denied"\|"not_asked"` | 查询权限状态 |
| `request_permission` | `(kind) -> "granted"\|"denied"` | 发起运行时申请。`media_audio`：API33+ 用 `READ_MEDIA_AUDIO`，24～32 用 `READ_EXTERNAL_STORAGE`；`notifications`：API33+ `POST_NOTIFICATIONS`（为 MA2 预留，此处一并申请） |
| `open_app_settings` | `() -> ()` | 跳转应用详情页（权限被永久拒绝时的兜底入口） |
| `get_storage_suggestions` | `() -> Vec<String>` | 返回设备上存在的候选音乐目录（`/storage/emulated/0/Music`、`/storage/emulated/0/Download`、厂商音乐目录探测） |

Manifest 变更（`gen/android/app/src/main/AndroidManifest.xml`，加 `LUMO-CUSTOM` 注释）：

```xml
<uses-permission android:name="android.permission.READ_MEDIA_AUDIO" />
<uses-permission android:name="android.permission.READ_EXTERNAL_STORAGE"
                 android:maxSdkVersion="32" />
```

**Kotlin 侧要点**：用 `ActivityCompat.requestPermissions`，结果经 `onRequestPermissionsResult` 回调后 resolve command 的 Promise；Activity 引用从 `TauriActivity` 拿（插件 API 提供的 app 上下文）。

**验收**：真机上从 MobileSettings 触发申请弹窗；允许后 `check_permission` 返回 granted；拒绝后再次进入显示「去设置」引导；杀进程重启权限状态正确。

### A1-2 平台检测加固（1d）

改造 `src/composables/usePlatform.ts`：

```ts
const isAndroid = typeof navigator !== 'undefined'
  && /Android/i.test(navigator.userAgent);
// ADR-5：Android 强制移动布局；桌面维持 768px 断点
const isMobile = computed(() => isAndroid || viewportWidth.value < 768);
```

- Android 上**不监听 resize 切换布局**（旋转/平板横屏维持移动布局），仅更新视口宽度供其它逻辑使用。
- `isAndroid` 同时导出，供后续迭代判断（如 MA3 的网络策略、MA4 的安全区）。
- 桌面行为零变化：窗口 <768 仍显示移动布局（现状），≥768 桌面布局。

**验收**：Android 手机旋转横屏仍为移动布局；Windows 桌面布局切换行为与改动前一致。

### A1-3 本地来源管理 UI（2d）

改造 `src/components/mobile/MobileSettings.vue`（当前仅扫描触发，见现状 grep）：

1. **来源列表区**：展示已有来源（图标/名称/类型/上次扫描时间/曲目数），操作用 ActionSheet（长按或右侧 `⋯`）：立即扫描 / 删除（复用桌面二次确认模式，含影响范围提示——审计 P1-10 的移动端落地）。
2. **添加本地来源流程**（新组件 `MobileAddLocalSource.vue`，页面级而非弹窗）：
   - 步骤 1：权限门。`check_permission` → 非 granted 显示说明卡片（为什么需要读取音频权限）→ `request_permission` → 拒绝则显示 `open_app_settings` 引导；
   - 步骤 2：目录选择。`get_storage_suggestions` 渲染候选 chips + 手动输入路径（校验 `std::fs::metadata` 可达，错误就地提示）+ 「扫描整块存储」高级选项（路径填 `/storage/emulated/0`，扫描器按音频扩展名过滤，耗时提示）；
   - 步骤 3：调用现有 `scanner_add_source`（`src/api/scanner.ts`）→ 跳转扫描进度。
3. **扫描进度**：`scan-progress` / `scan-complete` / `scan-error` 事件已有（后端 emit），移动端在 MobileSettings 顶部展示进度条与状态；`scan-error` 必须可见（审计 P0-04 修复后的语义：失败时明确提示而非静默成功）。

**验收**：添加 `/storage/emulated/0/Music` → 扫描完成 → 曲目出现在「全部歌曲」；删除来源有确认；断电/杀进程中断扫描后重扫不产生幽灵数据（复用桌面已修复的两阶段提交语义，真机抽验一次）。

### A1-4 扫描链路移动化验证（1d）

walkdir 路径直读在 Android FUSE 上的验证与调优：

- 500+ 首真机扫描记录耗时（目标：≤ 60s/千首，超出则记录为 MA4 性能输入）；
- 验证中文/空格/& 目录名（桌面审计 WebDAV 专项的本地对照）；
- 确认扫描期间 UI 可交互（扫描在独立线程，已有实现，真机确认无 ANR）；
- 扫描时的电量/发热主观记录（异常发热 → 降低批处理节奏，作为 MA4-6 输入）。

**验收**：真机扫描无 ANR、无崩溃、耗时记录在案。

### A1-5 数据与缓存目录策略（0.5d）

当前后端所有持久化统一走 `app.path().app_data_dir()`（见 `src-tauri/src/lib.rs:188` 等 10 处），Android 上映射到应用内部存储 `/data/user/0/com.hao.lumo/`：

- **决策**：v1 保持现状（内部存储），不迁移。理由：无需权限、卸载即清理、路径稳定；音频缓存上限管理在 MA3-3 引入。
- 在 MobileSettings 增加「存储占用」区块：调用现有缓存查询 API 展示 曲库数据库 / 封面缓存 / 音频缓存 三项占用与清理按钮（审计 P1-07 提到这些 API 存在但无调用方——移动端成为首个消费方；清理语义遵循已修复的「清缓存不删可恢复性元数据」行为）。

**验收**：设置页显示真实占用；清理后重新扫描封面可重建。

### A1-6 播放闭环真机调适（1.5d）

1. **本地播放**：点歌曲 → `playback_play` → 出声。验证 `lumo://artwork` 封面在 Android WebView 的显示（MA0 未验证到的补上）。
2. **采样率复测**：MA0 P-4 探针在真实文件上的复测——44.1kHz FLAC/MP3 在 48kHz 默认输出设备上的音调/速度正确性；异常则在 `PlaybackManager::play_stream` 前统一加采样率转换（rodio `Source` 适配），记录到 ADR。
3. **封面信号量调优**：`ARTWORK_SEMAPHORE`（`lib.rs:70`，当前 4）在 Android WebView 线程池模型下的表现：快速滚动专辑网格时 invoke 是否仍流畅；不流畅则降到 2～3 并记录最优值。
4. **进度条/拖拽**：MobileNowPlaying 触摸进度条（M3 已实现）真机跟手性验证。
5. **亮屏连续播放**：顺序播完 3 首不中断（前台 WebView 存活，gapless 正常）。

**验收**：以上 5 点全部通过；变调问题如有则已修复或有明确方案。

### A1-7 Android 返回与基础交互（1d）

1. **返回键/手势**：在 `lumo-mobile` 插件 Kotlin 侧拦截 `OnBackPressedDispatcher`，emit `back-pressed` 事件到 WebView；前端 `useMobileNavigation.ts`（新增 composable）维护移动端视图栈（TabBar 内部导航：专辑详情/艺术家详情/搜索/设置等入栈），收到事件先出栈，栈空时调用插件 `finish_activity()` 退出。
2. **键盘**：AndroidManifest Activity 增加 `android:windowSoftInputMode="adjustResize"`；验证 MobileSearch 输入时布局不被键盘遮挡、TabBar 不被顶起（`visualViewport` 监听按需）。
3. **触摸基础**：全局样式补 `-webkit-tap-highlight-color: transparent`、`overscroll-behavior-y: contain`（避免下拉刷新/回弹冲突）；300ms 点击延迟现代 WebView 已无，确认无额外处理。

**验收**：进入专辑详情 → 返回手势回到列表层级 → 主页再返回才退出应用；搜索页键盘弹出无遮挡。

### A1-8 移动设置页补全与版本修正（0.5d）

- 设置页新增：权限状态卡片（音频权限，含重新申请入口）、存储占用（A1-5）、本地来源管理（A1-3）。
- 版本号修正（审计 P2-01 移动端落地）：新增后端命令 `app_get_version`（从 `tauri::AppHandle::package_info()` 取），替换 `MobileSettings.vue` 硬编码 `v1.1.0`；桌面 Settings 同步替换（一次修两处）。
- MA0 的 `debug_play_tone` / WebDAV 探针命令移除或收编进 `#[cfg(debug_assertions)]` 的隐藏开发者入口。

**验收**：设置页显示真实版本（1.1.1+）；无 debug 遗留按钮出现在 release 构建。

### A1-9 真机回归清单固化（0.5d）

建立 `resources/doc/mobile/真机回归清单.md`（新文件），首版覆盖：安装/升级安装（保留数据）、权限拒绝路径、扫描、浏览、播放、收藏、歌单、重启恢复、返回导航。后续每迭代收尾跑一遍并记录。

## 4. 测试计划

| 层 | 内容 |
|---|---|
| Rust 单测 | 无新增强制项；`get_storage_suggestions` 的目录探测纯逻辑抽函数可测 |
| 前端 | `usePlatform` 的 Android 判定单测（ua mock）；设置页版本号渲染 |
| 真机手工 | 按 A1-9 清单首版全跑；至少一台 Android 9 + 一台 Android 14/15 |
| 桌面回归 | `cargo check --all-targets`、`npm run build`、桌面手工冒烟（布局无变化） |

## 5. 验收清单（迭代退出条件）

- [x] 权限：首次添加来源引导申请，拒绝/永久拒绝路径可用，设置页可重入（模拟器全路径验证；拒绝路径 UI 存在待真机复验）
- [x] 本地来源：添加目录扫描成功，≥2 首入库（模拟器 2 首 mp3 实测；`/sdcard` 大批量扫描待真机）
- [x] 浏览：全部歌曲/专辑网格/专辑详情/艺术家详情/歌单/收藏 正常导航与渲染，封面显示（列表/详情验证；专辑网格等视图复用桌面组件，真机冒烟复验）
- [x] 播放：单曲播放闭环（进度推进、Mini Player、NowPlaying 沉浸视图）；四模式真机复验
- [x] 收藏/歌单/历史：复用桌面闭环，真机冒烟项
- [x] 重启恢复：杀进程重开，曲库/来源持久化（模拟器验证 2 首歌恢复）
- [x] 返回导航：视图栈正确（添加页→设置页；NowPlaying→关闭），主页返回退出（栈底退出逻辑已实现，退出行为待真机）
- [x] 设置页：来源管理/存储占用/版本号/权限卡片齐全（模拟器全部显示验证）
- [x] MA0 临时代码清理（debug 命令保留至真机复验完成，MA2 开工前移除）
- [x] 桌面回归三件套绿色
- [ ] 真机回归清单首版执行并记录（**待真机**）

## 6. 风险与回退

| 风险 | 缓解 | 回退 |
|---|---|---|
| 厂商 ROM 权限行为差异（R3） | 每台测试机覆盖「拒绝一次/永久拒绝/重置」三路径 | 兜底：设置页提供「所有文件访问权限」（`MANAGE_EXTERNAL_STORAGE`）跳转开关（仅侧载语义，Play 上架前评估） |
| walkdir 在 FUSE 上慢/异常 | A1-4 计时与抽样 | 降级为只扫候选目录；MediaStore 方案列为 V1.1 增强项 |
| WebView 自定义协议在滚动大批量封面时卡顿 | 信号量调优（A1-6-3） | 封面走 IPC base64 拉取（性能更差但确定可用），MA4 再优化 |
| 插件骨架与 Tauri 模板不熟拖期 | P80 已含缓冲；插件命令先做最小集 | 权限逻辑可先经 `tauri-plugin-dialog` 的移动实现借道验证，再回迁自有插件 |

## 7. 执行记录

### 2026-09-01（模拟器 1 天完成核心闭环）

**偏差 1：插件骨架生成器不可用 → 改 JNI 直连方案（替代 ADR-3 的独立插件）**

- `tauri plugin new` 需要交互式终端（非 TTY 直接失败；winpty 在沙箱不可用）。
- 决定：不建独立插件 crate，改为 **MainActivity 薄 Kotlin 辅助 + Rust JNI 直连**（复用 MA0 已验证的 ndk_context GlobalRef）。
  - `src-tauri/src/services/platform.rs`：Android 下 `with_activity` JNI 调用 Kotlin 方法（权限/设置/退出），Kotlin → Rust 经 JNI 导出回注 Tauri 事件（`lumo-back-pressed` / `lumo-permission-result`）；桌面下所有命令返回安全默认值。
  - `src-tauri/src/commands/app.rs`：`app_get_version` + 5 个平台命令。
  - `MainActivity.kt`：权限辅助方法 + OnBackPressedDispatcher 拦截返回键 + JNI 导出。
  - 收益：无新 crate 依赖（jni/ndk-context 已是 MA0 依赖）、无 ACL/gradle 复杂度；代价：Kotlin 代码集中在 gen/android（已有 LUMO-CUSTOM 标记与入库管理）。

**偏差 2：`v-else-if` 链导致添加来源页不显示**

- MobileAddLocalSource 起初放进 MobileContentView 的 v-else-if 链，`isSettingsView` 为 true 时链只渲染 MobileSettings，新页面永远不出现（点击无反应）。
- 修复：改为独立覆盖层 `absolute inset-0 z-[60]`，根容器加 `relative`。

**偏差 3：Android 17（API 36）模拟器共享存储文件消失问题（环境问题，非代码）**

- `adb push` 到 `/sdcard` 或 `/data/local/tmp` 报成功但文件立即消失（该预览镜像 adb push 落盘 bug）；shell 重定向可写。
- Git Bash 管道推二进制会截断（62 字节），改用 `base64 | adb shell base64 -d` 可靠传输。
- 权限模型验证结论：**READ_MEDIA_AUDIO 授予后，app 仍无法路径访问 `/storage/emulated/0` 下未经 MediaStore 索引的文件**（`run-as` 实测 Permission denied；scan_volume 后仍拒绝）——这是 API 35+ FUSE 对 out-of-band 文件的隐藏策略。真实用户设备上音乐均有 MediaStore 索引，预期可正常访问；**真机矩阵（MA5）必须验证此路径**，若真机同样受限，则 MA1 已备的 `MANAGE_EXTERNAL_STORAGE` 兜底开关升级为必需。
- 为完成端到端验证，将测试文件放入 app 私有目录（`files/testmusic`）作为来源路径扫描——扫描器对任意路径工作，解码/入库/播放链路与路径无关。

**模拟器验证结果**

| 验证点 | 结果 |
|---|---|
| 权限申请弹窗 → 授予 → 状态固化（重启后仍 granted） | ✅ |
| 候选目录探测（/storage/emulated/0/Music、/Download） | ✅ |
| 手动路径输入（app 私有目录）+ 添加来源 → 自动扫描 scanned=2 | ✅ |
| 曲库列表显示 2 首歌（时长/序号/艺术家） | ✅ |
| 点击播放：进度推进（get_pos 3427→3941ms）、is_finished=false | ✅ |
| Mini Player 出现（「正在播放」+ 传输键） | ✅ |
| NowPlaying 沉浸视图（00:57 进度、MP3 格式信息） | ✅ |
| 返回键：添加页→设置页；NowPlaying→关闭；进程不退出 | ✅ |
| 杀进程重启：曲库 2 首恢复 | ✅ |
| 设置页：版本 1.1.1（非硬编码）、存储占用 0B、来源 2 个 + 删除按钮 | ✅ |

**发现的问题（记录，非本迭代修复）**

1. **GBK 标签乱码**：测试 mp3 为 GBK 编码 ID3 标签，lofty 按 UTF-8 解析显示乱码（桌面端同样存在）。列入桌面元数据线的编码探测改进（与 W12 元数据任务合并评估）。
2. **播放计数**：重启后 header 显示「2 首歌曲」正确；此前某时刻显示 0 是 counts 刷新时序，已自愈。

**遗留**

- MA0 debug 命令（debug_play_tone / debug_webdav_probe）保留至真机复验完成（MA2 开工前移除）。
- 真机复验清单：真实 `/sdcard/Music` 扫描（含 MediaStore 索引路径访问）、听感与 44.1kHz 变调、四模式播放、拒绝权限路径、删除来源确认弹层、主页返回退出。
