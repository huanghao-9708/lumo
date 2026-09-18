# Lumo 联网行为清单（NETWORK_BEHAVIOR）

> 文档状态：Draft v1（I0 / PC-001）
> 事实基线：Lumo v1.8.1，分支 `codex/commercial-maturity-roadmap`
> 核对方式：逐行审阅 `src-tauri/src/` 与 `src/` 的全部出网代码，行号为本文提交时的实际位置
> 适用范围：本文件是 Lumo 联网行为的**唯一事实源**。隐私政策、README、设置页文案一律引用本文件，不得各自维护第二套清单。

## 0. 总则

1. Lumo 是**本地优先**应用：曲库索引、播放、歌单、历史全部走本地 SQLite，离线可用。
2. **默认拒绝**：除用户主动配置的 WebDAV 来源外，所有在线增强能力默认关闭（见 §2）。
3. **无遥测**：全仓库不含任何 analytics / crash report / sentry / 埋点上报代码，不存在应用启动或心跳类请求。
4. 新增任何外部请求，必须在同一 PR 内更新本文件，否则视为 P1 缺陷（见 §6 评审规则）。

## 1. 外联目标总览

| # | 目标 | 域名 | 用途 | 触发方式 | 默认状态 | 关闭方式 |
|---|---|---|---|---|---|---|
| A | lrclib.net | `lrclib.net` | 在线歌词匹配 | 用户开启后，切歌时自动 | **关** | 设置页「在线歌词」开关 |
| B | 网易云音乐 | `music.163.com` + 响应内图片 CDN | 专辑/艺人封面搜索 | 用户开启后，缺封面时后台自动 | **关** | 设置页「在线封面」开关 |
| C | Apple iTunes | `itunes.apple.com` + `mzstatic.com` | 封面搜索兜底源 | 同 B | **关** | 同 B |
| D | AI 服务商 | 用户自填 `base_url` | AI 电台补全 | 用户手动点「生成」/「测试连接」 | **关** | `ai_settings.enabled` |
| E | WebDAV 曲库 | 用户自填 `root_uri` | 远程扫描、流播、缓存 | 用户操作 + 播放自动 | 无默认值 | 删除来源或停用 |
| F | WebDAV 备份 | 用户自填 `sync_config` | 整库快照上传/恢复 | 上传手动；**打开设置页会自动 PROPFIND 检查** | `enabled=false` | 关闭备份恢复开关 |
| G | GitHub API | `api.github.com` | 移动端检查新版本 | 仅用户手动点按钮 | 无开关 | 不点即不请求 |

以上七类**穷尽**了当前代码中的出网路径。未列出的域名（Deezer、MusicBrainz）仅存在于注释（`services/cover.rs:9`、`models.rs:48`），无实现。

## 2. 逐项明细

### A. 在线歌词 — lrclib.net

- **入口**：`library_get_lyrics`（`commands/library.rs:206`）→ `commands/library.rs:274` 构造 `https://lrclib.net/api/get`。
- **前端调用点**：`src/stores/player.ts:1180`，切歌时自动触发。
- **请求参数（发出字段）**：曲名、艺术家、专辑、时长（`commands/library.rs:275-278`）。**不含**文件路径、设备标识。
- **User-Agent**：`LumoMusicPlayer/1.0.0`（`:281`）。
- **门禁**：`allow_online != Some(true)` 直接返回（`:245`）。开关 `lumo_fetch_lyrics`，`src/stores/ui.ts:131`，**默认关闭**。
- **落地**：`lyrics` 表，`source='lrclib'`（`:321,324`）。
- **超时**：**未设置**（`:280-283`）——缺陷 D-06。
- **已知与文案不一致处**：设置页文案为「未命中本地歌词时」才联网，但代码在本地仅有纯文本（无同步时间轴）歌词时同样会发起请求（`:237-241`）。文案需收紧或代码放宽判定。

### B. 在线封面主源 — music.163.com（**此前未向用户声明**）

- **入口**：`https://music.163.com/api/search/get`（`services/cover.rs:88` 专辑、`:130` 艺人）、`https://music.163.com/api/artist/{id}`（`:137`）。
- **请求特征**：固定 `Referer: https://music.163.com`（`:15`）、桌面浏览器伪装 UA（`:16`）。这两项是为绕过该服务的防盗链，属于**对第三方非公开接口的依赖**，见 §4 风险 R-01。
- **发出字段**：专辑标题（含拼接的艺人名，`:84`）、艺人名称。
- **二次请求**：从响应中取 `picUrl` 追加 `?param=1500y1500` / `1200y1200` 后再次 GET（`:96-99`、`:143-144`），并把 `http://` 强制改写为 `https://`（`:49`）。**因此图片实际下载主机由服务端响应决定，无法预先穷举进域名白名单**——只能声明为「网易云及其 CDN」。
- **触发**：`library_fetch_missing_album_cover`（`commands/library.rs:791`）/ `..._artist_cover`（`:903`），经 `tokio::spawn` 后台执行并 emit 事件；前端 `player.ts:1307,1460` 在列表加载时自动排队。
- **门禁**：`allow_online`（`commands/library.rs:795,907`）；开关 `lumo_fetch_covers`（`ui.ts:132`），**默认关闭**。
- **落地**：`artwork` 表 + `app_data_dir/artworks/<sha256>.<ext>`（`cover.rs:734-748`），并回写 `albums.cover_artwork_id` / `artists.avatar_artwork_id`。
- **失败抑制**：会话级负缓存避免对同一未命中项反复请求（`:700,828`）。

### C. 在线封面兜底源 — itunes.apple.com

- **入口**：`https://itunes.apple.com/search?term=...&entity=album&limit=1`（`cover.rs:110,156`）；命中后把 `artworkUrl100` 改写为 `100000x100000bb` 下载（`:119,165`），实际图片主机为 `mzstatic.com`。
- 与 B 共用同一 client（含伪装 UA）与 20s 超时（`:17,31`）。
- **这是设置页当前唯一声明了的封面服务商**（`Settings.vue:220`、`MobileSettings.vue:378`），与真实链路不符，见 §4 风险 R-02。

### D. AI 电台 — 用户自备服务商

- **入口**：`POST {base_url}/chat/completions`（`services/ai.rs:353`）、`GET {base_url}/models`（`:513`，测试连接）。
- **base_url 来源**：`ai_settings` 表，DDL 默认 `''`（`db.rs:654`），前端仅提供 placeholder 不提供默认值（`Settings.vue:250`）；为空时直接拒发（`ai.rs:350`）。**即：AI 能力在未配置 Key/URL 时零出网。**
- **发出字段**：`model`、`temperature`、`messages`；user prompt 内含候选歌曲的 `id | 标题 | 艺人 | 播放次数`（`ai.rs:334-337`）。**播放次数即收听偏好，属个人数据**，须在 UI 明示（见 §4 R-03）。
- **鉴权**：`Authorization: Bearer <key>`（`:374,515`）。Key 存系统钥匙串（桌面）或机器绑定加密文件（Android），DTO 只回 `has_key`（`models.rs:492`），不回传明文。
- **超时**：生成 90s（`:26,356`）、测试连接 10s（`:509`）。
- **开关**：`ai_settings.enabled`，DDL 默认 `0`（`db.rs:653`）。
- **缺陷（P1）**：`ai_test_connection`（`commands/ai.rs:67-74`）**未校验 `enabled`**，在 AI 关闭状态下点击「测试连接」仍会带 API Key 外发。修复前，本项不满足「关闭后不产生对应请求」的验收标准。
- **日志**：`ai.rs:390-391` 记录服务商响应前 200 字符——若服务商回显请求内容，可能把曲目信息写入日志（见 §5）。

### E. WebDAV 远程曲库

- **base_url 来源**：用户填写的 `sources.root_uri`（`commands/scanner.rs:219`），**无内置默认值**。
- **方法与端点**：`PROPFIND`（Depth 0/1，`services/webdav.rs:103,164`）、`GET` + `Range`（`:211,428`）、`PUT`、`MKCOL`（`:323`）；`basic_auth`（`:85-91`）。
- **触发**：扫描由用户点击 `source_scan`（`:239`）；扫描期对每个远端文件做 Range GET 以解析标签（`services/scanner.rs:526-530`）。播放远程曲目时流播，并**无条件在后台整曲下载缓存**（`commands/playback.rs:283-286`、`commands/queue.rs:69,143`）→ `app_data_dir/audio_cache/<id>`（`services/cache.rs:60,70`，上限 2GB，`playback.rs:240`）。该后台下载**没有独立开关**，见 §4 R-04。
- **超时**：常规 client 连接 10s + 总 60s（`webdav.rs:41-45`）；**下载 client 无总超时**（`:46-49`），缺陷 D-07。
- **降级**：`root_uri` 为空时探测退化到 `http://localhost`（`:96-100`），属内部探针行为，不对外发数据。
- **TLS**：使用 rustls + `danger` 之外的默认校验，全仓无 `danger_accept_invalid_certs`，因此自签证书会连接失败（不做静默放行）。服务端差异与降级口径见 I3 的 `WEBDAV_COMPATIBILITY.md`。

### F. 数据备份恢复（对外称谓，代码模块名仍为 `sync`）

- **上传**：`PUT <remote_path>/lumo.sqlite`，整份数据库快照（`services/sync.rs:199-213`）。
- **恢复**：`GET` 后覆盖本地（`:235-241`）；**语义为整库替换，不做实体级合并、无冲突处理**，因此本计划统一称「备份恢复」而非「同步」。
- **凭据剔除**：快照生成前已清空 `sync_config.password_encrypted` 与 `sources.credential_ref`（`:166-172`）——**远端快照不含任何口令/密钥**。
- **仍包含的数据**：全部曲库元数据、**本地绝对/相对路径**、播放历史、歌单、收藏。跨设备漫游意味着这些数据上传到用户自己的 WebDAV，须在 UI 明确告知（§4 R-05）。
- **门禁**：上传/恢复受 `config.enabled` 拦截（`commands/sync.rs:75,93`），前端默认 `enabled=false`（`src/stores/sync.ts:26`）。
- **缺陷（P2）**：打开设置页即自动执行 `checkRemote()`（PROPFIND，`Settings.vue:124-130`），并非用户显式动作，与「用户授权联网」原则相冲突。

### G. GitHub 更新检查

- **入口**：`https://api.github.com/repos/huanghao-9708/lumo/releases/latest`（`src/components/mobile/MobileSettings.vue:147`），成功后 `window.open` 跳转发布页（`:172`）。
- **仅移动端、仅手动点击按钮**（`:610`）触发；桌面「关于」页只读本地版本（`Settings.vue:469-484`）。
- **无自动检查、无 tauri updater 插件**（`Cargo.toml`、`tauri.conf.json` 均无 updater endpoints），即应用不会自行下载或安装更新。
- 本项未在设置页声明，属轻微缺口（§4 R-06）。

## 3. 连接层事实

| 项 | 现状 | 证据 |
|---|---|---|
| TLS 后端 | rustls 0.23 + `ring`（纯 Rust，为 Android 交叉编译所选），进程内安装默认 provider | `Cargo.toml:37-42`、`lib.rs:230-234` |
| 证书校验 | 全程开启，无跳过开关 | 全仓无 `danger_accept_invalid_certs` |
| 系统代理 | reqwest 启用 `system-proxy` feature，**所有外联可能经由系统代理/PAC** | `Cargo.toml:38`；未调用 `no_proxy()` |
| User-Agent | 三套并存：封面用浏览器伪装 UA、歌词用 `LumoMusicPlayer/1.0.0`、WebDAV/AI/GitHub 用 reqwest 或 WebView 默认 UA | `cover.rs:16`、`commands/library.rs:281` |
| 前端 fetch | 只访问本地协议 `lumo://artwork`（`useArtworkSrc.ts:99`、`artworkCache.ts:121`）与 §2G 的 GitHub；不直连其他第三方 | — |
| 窗口 CSP | `csp: null`（未设置），I2 待收口 | `tauri.conf.json:25-27` |

## 4. 隐私风险登记（转 I2 处置）

| ID | 风险 | 严重度 | 归属迭代 |
|---|---|---|---|
| R-01 | 依赖网易云非公开接口并伪装浏览器 UA，服务随时可能失效或构成合规争议 | P1 | I2 定策略（公开声明 / 换开放源） |
| R-02 | 设置页只声明 iTunes，未声明实际主源网易云与 lrclib，对外披露不完整 | P1 | **I0 本次修正文案** |
| R-03 | AI 请求携带收听次数，未向用户说明该字段外泄 | P1 | I0 文案 + I2 政策 |
| R-04 | 远程播放无条件后台整曲落盘，无独立开关，占磁盘且不受隐私开关约束 | P2 | I3 缓存治理 |
| R-05 | 备份快照含本地路径与播放历史，UI 未充分告知 | P1 | I3 恢复 UI 重做 |
| R-06 | GitHub 更新检查未在联网声明中出现 | P2 | I0 本次补声明 |
| R-07 | **两个隐私开关存 localStorage 而非数据库**：换设备/清 WebView 即重置，且不随备份迁移；后端门禁依赖前端传入 `allow_online`，直连 IPC 可绕过（`ui.ts:126` 自称「双保险」） | P1 | I2（开关落库 + 后端独立判定） |
| R-08 | 打开设置页自动 WebDAV PROPFIND | P2 | I2 |
| R-09 | 系统代理下所有元数据查询可能经第三方代理节点，未在文档披露 | P2 | I2 |

## 5. 日志与敏感信息

- 日志初始化：`tracing_subscriber::fmt::init()`（`lib.rs:228`），**仅 stdout / logcat，无文件 appender**；IPC 追踪 `ipc_trace.rs:48-57` 只记命令名与耗时，不落参数。
- **已确认的泄露点**：
  1. `webdav.rs:434,463,467` 打印完整请求 URL；若用户把账号写进 URL 的 userinfo 段，口令会进日志。
  2. `webdav.rs:60-68` `describe_reqwest_error` 把含 URL 的错误原文透传到前端 toast，等于把上述内容显示给用户并可被截图传播。
  3. `ai.rs:390-391` 记录 AI 响应前 200 字符。
- 未发现明文 API Key 或口令进入日志的路径（Key 仅在请求头，`ai.rs:374`）。日志脱敏与文件落盘方案归 I5（诊断包）一并处理。

## 6. 变更控制规则

1. 新增出网必须随 PR 提交本文件的更新，并在 §1 表格占一行；评审时缺此项即打回。
2. 新增外联默认关闭；任何「应用启动即请求」的设计一律视为 P0 阻断。
3. 本文件与隐私政策、设置页文案不一致时，**以代码为准并立即修正文档**，不得为了维护旧承诺而隐瞒真实行为。
4. 域名清单中「由服务端响应决定的动态主机」（网易云 CDN、mzstatic）在对外政策里以「及其内容分发网络」表述，不假装可枚举。

## 7. 待人工复核

- reqwest 0.13 的 `system-proxy` 在 Windows 上是否读取注册表并执行 PAC（影响 R-09 的披露粒度）。
- §2A 中「本地纯文本歌词是否应视为已命中」的产品判定。
