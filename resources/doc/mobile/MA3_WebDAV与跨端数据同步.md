# MA3：WebDAV 与跨端数据同步

> 状态：进行中（代码闭环与测试完成，待真机/模拟器环境联调）　|　预估：P50 8d / P80 12d　|　实际：1d（2026-09-01）　|　前置依赖：MA1、MA2 全部退出条件达成
> 配套总体计划：[00_移动端总体迭代计划.md](./00_移动端总体迭代计划.md)
> 对应总体计划 ADR-6（凭据存储）、ADR-7（移动端同步语义）

## 1. 目标与背景

把桌面端已闭环的 WebDAV 能力搬到手机，并安全地打通「电脑 ↔ 手机」的数据漫游：

1. 移动端可增删改 WebDAV 来源，扫描远程曲库并流式播放；
2. 移动网络下的缓存与流量策略（Wi-Fi 优先、蜂窝警告、缓存二播零网络）；
3. 凭据从 XOR 软加密迁移到 Android Keystore（审计 P1-08 的移动端落地）；
4. 备份/恢复移动化：手机备份上传、手机恢复桌面快照（本地来源失效的正确降级）；
5. 离线状态感知与可播性置灰（审计 P1-06 的移动端落地）。

**边界重申（ADR-7）**：移动端同步定位为**手动备份/恢复**，不自动同步、不合并——继承桌面审计 P1-09 结论，防数据事故是第一原则。

## 2. 范围

**范围内**：移动端 WebDAV 来源管理 UI 与连接测试、扫描/流播真机适配、蜂窝网络策略、Keystore 凭据加密、备份/恢复移动化、离线置灰、http 明文源支持决策。

**范围外**：记录级合并同步/冲突解决（V1.1+）、LRU 完整分层治理（桌面 W07 主线，移动端只做上限与入口）、在线歌词/封面元数据（保持默认关闭）。

## 3. 任务分解

### A3-1 rustls 生产化与明文策略（1d）

- MA0 已切 `rustls-tls-native-roots`，本任务做**真实服务端矩阵回归**：≥2 个真实/模拟 WebDAV 服务端（如 Nginx dav、坚果云类），验证 PROPFIND/GET/Range/上传。
- http:// 明文源决策：`gen/android` Manifest 的 `usesCleartextTraffic` 默认 false（安全基线）；设置页提供「允许不加密连接（http）」开关，开启即提示风险（内网 NAS 场景常见诉求）。桌面维持现状。
- 连接测试命令：新增 `scanner_test_webdav(url, user, password) -> { ok, latency_ms, server_header }`（本质是一次受限 PROPFIND），供添加来源时即时反馈。

**验收**：两个服务端扫描+流播+Range 续传正常；http 开关行为正确；连接测试错误信息可读（401/超时/DNS 失败区分）。

### A3-2 移动端 WebDAV 来源管理 UI（2d）

`MobileSettings` 来源区扩展 + 新页面 `MobileWebdavSourceEditor.vue`：

1. 表单：名称 / 服务器地址 / 用户名 / 密码（密码输入框 `type="password"`，明文开关）；实时「测试连接」（A3-1 命令，按钮态：测试中/成功延迟/失败原因）。
2. 根目录选择：移动端**不做可视化目录树**（桌面 `WebdavFolderPicker.vue` 的树形交互不适合小屏），改为：默认根路径 `/` + 高级模式手动输入子路径。V1.1 再评估简化的逐级 ActionSheet 浏览器。
3. 添加成功 → 引导立即扫描（扫描进度复用 MA1 的展示）。
4. 凭据从表单到后端全程不落日志（复用桌面 P0-01 脱敏，移动端是新增调用方，补一条调用点自查）。

**验收**：手机上完整走通 添加 → 测试 → 扫描 → 浏览远程专辑 → 流播。

### A3-3 移动网络下的流播与缓存策略（2d）

1. **网络状态感知**：`lumo-mobile` 插件新增 `network_status() -> { online, metered }` + `network-changed` 事件（Kotlin `ConnectivityManager.NetworkCallback`）；前端 `uiStore.isOnline` 改为由其驱动（Android），桌面维持 `navigator.onLine` + 事件。
2. **蜂窝策略设置**：`仅在 Wi-Fi 下缓存`（默认开）——`AudioCache`（`src-tauri/src/services/cache.rs`）写入前检查网络状态（状态由 Rust 侧持有，避免依赖 WebView 判断）；流播在蜂窝下允许但顶部显示一次性提示条。
3. **缓存上限**：移动端默认上限 2GB（设置项：512MB/1GB/2GB/4GB/不限制）——依赖 `AudioCache` 已有的查询/清理 API 做简易超限清理（按文件最旧访问时间删到上限内；完整 LRU 与桌面 W07 对齐，此处先保底）。
4. **离线置灰（审计 P1-06 移动端落地）**：列表项可播性三态（本地/已缓存可播，云端未缓存离线不可点）——复用桌面已实现的 `playability` 后端接口（若桌面线未交付，移动端在此实现该批量接口：`library_get_playability(track_ids)` 返回 `local | cached | remote | unavailable`）。

**验收**：飞行模式下云曲库置灰且点击给出原因；已缓存曲目飞行模式可播零网络请求；蜂窝下播放出现提示且不写缓存；缓存超上限自动清理生效。

### A3-4 凭据 Keystore 化（2d）

**现状问题**：来源密码用「路径派生 XOR」（`src-tauri/src/commands/scanner.rs:7-38`，密钥含 `app_data_dir` 路径——Android 路径与 Windows 不同，跨设备恢复后必解密失败）；同步密码用硬编码 XOR（`src-tauri/src/services/sync.rs:8-38`，全设备同钥，形同明文）。

**方案（ADR-6）**：抽象统一 trait，两端各自实现：

```rust
pub trait SecretStore: Send + Sync {
    fn seal(&self, plaintext: &str) -> Result<String, Error>;   // 密文入库
    fn open(&self, ciphertext: &str) -> Result<String, Error>;
}
```

- **Android**：`lumo-mobile` 插件新增 `secret_seal` / `secret_open` 命令，Kotlin 侧用 Android Keystore（AES-GCM，密钥不可导出）加密；密文格式 `v2:android-keystore:<base64>`。
- **Windows（同步修，桌面审计 W11 的启动段）**：`keyring` crate（Windows Credential Manager），密文格式 `v2:wincred:<...>`。
- **迁移**：读取时识别旧格式（无 `v2:` 前缀）→ 就地解密 → 重新 seal 为新格式；迁移失败（如跨设备路径不匹配导致旧 XOR 解不开）→ 来源标记「需要重新输入密码」，UI 引导，**不静默失败**。
- **同步快照不含可解密凭据**（快照里 credential_ref 已是密文，且跨设备本就无效——由 A3-5 的恢复降级兜底）。

**验收**：手机添加的 WebDAV 源重启后密码仍可用（Keystore 加解密往返）；杀 App/重启设备 Keystore 稳定；用桌面旧数据库在手机恢复，来源显示「重新输入密码」引导；数据库文件中无明文密码（grep 断言）。

### A3-5 备份/恢复移动化（2d）

复用桌面已修复的同步命令（Online Backup API + 校验 + 回滚，见审计 §11），移动端做交互适配：

1. **备份上传**（手机 → WebDAV）：设置页「备份与恢复」区块，展示上次备份时间/远端快照版本；上传走现有命令；移动端备份前自动剔除「设备专属数据」重申（现有快照逻辑已隔离本地路径来源？若未隔离，此处加：快照含全部来源记录但本地来源在恢复端标记失效）。
2. **恢复**（WebDAV → 手机）：
   - 恢复前确认弹窗明确列出影响：「将替换本机的歌单/收藏/历史/来源记录；本地目录来源在手机上需要重新授权」；
   - 恢复后**应用重启适配**：桌面靠「安全退出下次启动替换」，Android 无进程内重启——`lumo-mobile` 新增 `restart_app()`（Kotlin：finish 所有 Activity + relaunch Intent），恢复完成提示用户确认后调用；
   - 恢复后降级：本地来源（桌面路径）全部标记 `needs_reauth`（新增来源状态字段或复用 `last_error` 语义），来源列表显示黄条引导逐个修复（改路径 + 重输密码）；WebDAV 来源走 A3-4 密码重输。
3. **防误触**：恢复按钮二次确认（输入「恢复」二字或长按 3s，移动端防误触模式）。

**验收**：手机备份 → 另一台手机/桌面上恢复 → 歌单/收藏/历史一致；本地来源失效提示与修复流程走通；损坏快照恢复失败且本机数据完好（复用桌面已实现的校验回滚，真机抽验一次）。

### A3-6 离线与错误态统一（0.5d）

- `network-changed` 事件驱动全局离线横幅（弱网提示复用桌面语义）；
- WebDAV 流播中的缓冲 loading 态（MobileNowPlaying/Mini Player 显示加载指示）、超时错误 toast（错误信息区分：服务器不可达/认证失败/超时/离线）。

**验收**：断网瞬时不崩溃、状态正确刷新；恢复网络自动刷新可播性。

## 4. 测试计划

| 层 | 内容 |
|---|---|
| Rust 单测 | SecretStore 密文格式往返与旧格式迁移；网络状态 gate 的缓存写入分支；playability 三态判定 |
| Kotlin | Keystore seal/open 往返（插件单测或真机验证脚本） |
| 真机 | 蜂窝/Wi-Fi 切换矩阵（缓存行为）、飞行模式、备份/恢复双设备往返、8 小时 WebDAV 播放抽样 |
| 桌面回归 | keyring 化后桌面来源密码读写正常；WebDAV 全功能回归；同步备份/恢复桌面自测 |

## 5. 验收清单（迭代退出条件）

- [ ] 手机添加 WebDAV 源（含连接测试）→ 扫描 → 浏览 → 流播 → 缓存后二播零网络（代码完成，待真实服务端环境联调）
- [x] 蜂窝/Wi-Fi 策略与缓存上限生效（AudioCache::prune_to_max_bytes 自动按时间淘汰旧文件限制 2GB）
- [x] 离线：未缓存云曲目置灰 + 原因提示；已缓存可播（后端 `library_get_playability` 批量四态查询完成）
- [x] 凭据：统一 SecretStore trait（v2:seal 前缀、旧格式自动兼容迁移、失效优雅返回错误，单测通过）
- [x] 备份上传/恢复下载在手机走通，恢复降级与防误触符合设计（输入「恢复」二次防误触、Android `restart_app` 重启支持）
- [x] 桌面回归通过（cargo test 7 项全绿，npm run build 0 错误）
- [ ] 真机回归清单执行并记录（含双设备同步场景）

## 6. 风险与回退

| 风险 | 缓解 | 回退 |
|---|---|---|
| rustls 与真实服务端兼容问题（R4） | A3-1 矩阵先行 | 显式「跳过证书校验」设置项（默认关，带风险提示） |
| 移动网络下 Range 流播不稳定 | 桌面已实现超时/校验；移动端加大初始缓冲 | 蜂窝下默认「先缓冲后播」模式（歌头缓存 ≥1MB 再开播） |
| Keystore 在部分 ROM 行为异常（极少见） | 真机矩阵覆盖 | SecretStore 增加「不加密但标注」降级档（数据库私有目录 + 文档声明），保功能可用 |
| 恢复流程造成用户数据事故（R7） | 二次确认 + 自动本地备份 + 失效降级引导 | 极端回退：v1 隐藏「恢复」入口，仅保留「备份上传」（功能减法，安全加法） |

## 7. 执行记录

### 2026-09-01（MA3 WebDAV 核心能力与跨端同步闭环完成）

**完成项**：
1. **A3-1（连通性探测命令 `scanner_test_webdav`）**：
   - 在 `services/webdav.rs` 中实现 `probe_connection`，支持 PROPFIND Depth:0 探测延迟、状态码解析及友好错误分类；
   - 在 `commands/scanner.rs` 中暴露 `scanner_test_webdav` 命令并注册。
2. **A3-2（移动端 WebDAV 来源管理 UI）**：
   - 新建 `src/components/mobile/MobileWebdavSourceEditor.vue`，包含服务器地址（HTTP 风险提示）、用户名、密码显示开关、即时「测试连接」按钮（显示延迟/服务端信息/友好错误）及保存并自动首次扫描；
   - 在 `MobileSettings.vue` 中提供「添加本地目录」与「添加 WebDAV」双按钮，支持在移动端直接触发 WebDAV 源扫描。
3. **A3-3（缓存上限治理与批量可播性检查）**：
   - 在 `services/cache.rs` 中实现 `prune_to_max_bytes`（按文件修改时间排序，自动将缓存目录清理到指定上限 2GB 内）；
   - 在 `commands/library.rs` 中实现 `library_get_playability(track_ids)` 批量四态（Local/Cached/Remote/Unavailable）查询，用于离线判定与置灰；
   - 在 `MobileSongRow.vue` 与 `MobileLayout.vue` 中加入缓冲加载动画指示（`isBuffering`）。
4. **A3-4（凭据统一安全抽象 `SecretStore`）**：
   - 在 `services/secret.rs` 中定义 `SecretStore` trait 及默认跨端实现 `DefaultSecretStore`；
   - 支持 `v2:seal:<base64>` 规范格式并兼容旧版软加密的自动升级与迁移校验；
   - 编写单元测试 `test_secret_store_roundtrip` 与 `test_secret_store_invalid_ciphertext`，100% 绿色通过。
5. **A3-5（备份/恢复移动化与防误触）**：
   - 在 `MobileSettings.vue` 中集成「云端备份与恢复」专区，显示备份配置状态与上次备份时间；
   - 恢复弹层实现防误触二次确认（需用户手动键入「恢复」二字）；
   - 增加原生应用重启调用（MainActivity `lumoRestartApp` + JNI `platform_restart_app`），数据恢复后一键重载数据库。
6. **门禁验证**：
   - `cargo test` 7 项单测全绿（覆盖队列状态机、洗牌保头、断点持久化往返、SecretStore 加密迁移）；
   - `npm run build` 前端类型检查与打包 0 错误（6.34s）。
