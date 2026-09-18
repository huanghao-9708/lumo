# Lumo 联网行为清单（NETWORK_BEHAVIOR）

> 文档状态：Draft v2（I0/PC-001 建立，I3/DATA-002 同步修复后更新）
> 事实基线：Lumo v1.8.1，分支 `codex/commercial-maturity-roadmap`
> 核对方式：逐行审阅 `src-tauri/src/` 与 `src/` 的全部出网代码。**行号是本文核对时点的位置**，
> 代码改动后会漂移——定位请以括注里的函数名为准；机器门禁只校验「文件是否登记」
> （`scripts/check-network-registry.mjs`），不校验行号。
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

- **入口**：`library_get_lyrics`（`commands/library.rs:321`）→ `:407` 构造 `https://lrclib.net/api/get`。
- **前端调用点**：`src/stores/player.ts:1180`（切歌时自动触发，仅当 `uiStore.fetchLyricsOnline` 为真才传 `allowOnline=true`）→ `src/api/library.ts:190`。
- **请求参数（发出字段）**：`track_name`、`artist_name`、`album_name`、`duration`（`:407-418`）。**不含**文件路径、设备标识、曲目 ID。
- **User-Agent**：`LumoMusicPlayer/1.0.0`（`:421`）。
- **门禁**：`allow_online != Some(true)` 直接返回本地结果（`:367`）。开关 `lumo_fetch_lyrics`，`src/stores/ui.ts:127`，**默认关闭**。
- **落地**：`lyrics` 表，`source='lrclib'`（`:463,466`）。
- **超时**：15s（`LRCLIB_TIMEOUT_SECS`）。I3 之前**完全没有超时**——该请求挂在切歌路径上，
  第三方黑洞化会拖住 IPC，现已补齐；仍存在的问题是**每次切歌都可能重试**（歌词侧无负缓存）。
- **本地命中判定**：仅当本地已有 `synced = 1`（带时间轴）歌词才短路返回（`:355-360`）；
  本地只有纯文本歌词时仍会联网补时间轴。设置页文案已按此口径收紧（I0）。

### B. 在线封面主源 — music.163.com（**此前未向用户声明**）

- **入口**：专辑 `services/cover.rs:94`、艺人 `:140` 的 `https://music.163.com/api/search/get`；
  艺人详情 `:149` 的 `https://music.163.com/api/artist/{id}`。
- **请求特征**：固定 `Referer: https://music.163.com`（`:15`，用于 `:41`）、
  伪装桌面浏览器 UA `Mozilla/5.0 (Windows NT 10.0; Win64; x64) LumoPlayer`（`:16`）。
  这两项是为绕过该服务的防盗链，属于**对第三方非公开接口的依赖**，见 §4 风险 R-01。
- **发出字段**：搜索关键词（专辑标题拼接艺人名 / 艺人名称）。
- **二次请求**：从响应取 `picUrl` 追加 `?param=1500y1500`（专辑 `:104`）/ `?param=1200y1200`（艺人 `:159`）
  后再次 GET，并把 `http://` 强制改写为 `https://`（`:48-50`）。
  **因此图片实际下载主机由服务端响应决定，无法预先穷举进域名白名单**——只能声明为「网易云及其 CDN」。
- **触发**：`library_fetch_missing_album_cover`（`commands/library.rs:1053`）/ `..._artist_cover`（`:1189`），
  经 `tokio::spawn` 后台执行并 emit 事件；前端在列表加载时自动排队。
- **门禁**：`allow_online`（`:1062` / `:1198`）；开关 `lumo_fetch_covers`（`ui.ts:128`），**默认关闭**。
- **落地**：`artwork` 表（内容寻址、按 `content_hash` 去重）+ 本地图片缓存，并回写
  `albums.cover_artwork_id` / `artists.avatar_artwork_id`。
- **失败抑制**：会话级负缓存 `COVER_ATTEMPTS`，TTL 600s（`commands/library.rs:1359-1361`），同一目标 10 分钟内只尝试一次。

### C. 在线封面兜底源 — itunes.apple.com

- **入口**：`https://itunes.apple.com/search`（专辑 `cover.rs:120`、艺人 `:175`，参数 `term` / `entity=album` / `limit=1`）；
  命中后把 `artworkUrl100` 的 `100x100bb` 改写为 `100000x100000bb` 下载（`:129` / `:184`），
  实际图片主机为 `mzstatic.com`。
- 与 B 共用同一 client：伪装 UA + 20s 总超时（`:17`、`:30-32`）。
- **设置页文案已改为同时声明网易云与 iTunes**（`Settings.vue:220`）；I0 之前只声明了 iTunes，
  与真实链路不符（§4 R-02 文案侧已闭环，仅待产品复核措辞）。

### D. AI 电台 — 用户自备服务商

- **入口**：`POST {base_url}/chat/completions`（`services/ai.rs:390`）、`GET {base_url}/models`（`:609`，测试连接）。
- **base_url 来源**：`ai_settings` 表，DDL 默认 `''`（`db.rs` V9 版本块），前端仅提供 placeholder 不提供默认值；
  未填写 Base URL 或模型名时 `validate()` 直接拒发（`services/ai.rs:754-765`）。**即：AI 能力在未配置时零出网。**
- **发出字段**：`model`、`temperature`、`messages`；user prompt 内含候选歌曲的
  `id | 标题 | 艺人 | 播放次数`（`services/ai.rs:360-367`，上限 `MAX_TRACKS = 40`，`:24`）。
  **播放次数即收听偏好，属个人数据**，须在 UI 明示（见 §4 R-03，I0 已改文案）。
- **鉴权**：`Authorization: Bearer <key>`（`:411` 生成、`:611` 测试连接）。Key 存系统钥匙串（桌面）
  或机器绑定加密文件（Android），DTO 只回 `has_key`（`models.rs:499`），不回传明文。
- **超时**：生成 90s（`LLM_TIMEOUT_SECS`，`:26` 定义、`:393` 应用）。
- **开关**：`ai_settings.enabled`，DDL 默认 `0`（`db.rs` V9 版本块）。
- **未开启时零出网（I3/G-06 已修复）**：`ai_test_connection`（`commands/ai.rs`）先过
  `AiSettings::validate()` 门禁，未开启/未配置时直接返回失败文案，
  **不产生任何请求、不带 Authorization**。此前在 AI 关闭状态下点「测试连接」
  仍会带着 API Key 外发，本项验收标准现已成立。
- **日志**：`services/ai.rs` 记录服务商响应前 200 字符——若服务商回显请求内容，可能把曲目信息写入日志（见 §5）。

### E. WebDAV 远程曲库

- **base_url 来源**：用户填写的 `sources.root_uri`（添加来源时写入，见 `commands/scanner.rs` 的 `source_add_*`），**无内置默认值**。
- **方法与端点**：`PROPFIND`（Depth 0/1，`WebdavClient::propfind` / `probe_connection`）、`GET` + `Range`（`probe_range_support`、`HttpRangeReader::read`）、`PUT`（`put_file`）、`MKCOL`、`MOVE`（`move_file`，I3 新增）、`DELETE`；`basic_auth`（`apply_auth`）。
- **触发**：扫描由用户点击 `source_scan`（`commands/scanner.rs`）；扫描期对每个远端文件做 Range GET 以解析标签（`services/scanner.rs`）。播放远程曲目时流播，并**无条件在后台整曲下载缓存**（`commands/playback.rs::spawn_background_cache_download`，队列自动切歌路径 `commands/queue.rs` 同样触发）→ `app_data_dir/audio_cache/<media_file_id>`（`services/cache.rs::AudioCache`，上限 `DEFAULT_MAX_BYTES` = 2GB，超出按最久未用淘汰）。该后台下载**没有独立开关**，见 §4 R-04。
- **完整性**：下载落盘前后都比对 `media_files.file_size`（`AudioCache::store_from_webdav` / `get_cached_path`），大小不符即丢弃并重新下载，不会把截断文件固化成坏缓存（I3/G-10）。服务端未上报大小时退化为「非空即有效」。
- **超时**：两个客户端并存（`WebdavClient::new`）——
  `client`：连接 10s + **总** 60s，用于 PROPFIND / MKCOL / DELETE / Range 探测 / Range 读；
  `bulk_client`：连接 10s + **无总超时**（reqwest blocking 无读超时能力，改用 TCP keepalive
  空闲 30s / 间隔 10s 探测死连接），用于整文件下载与快照 PUT，
  大文件不会再被 60s 掐断（I3/G-09）。构建失败退回默认客户端时会 `tracing::error!` 留痕。
- **重试与退避（I3/G-12）**：重试只发生在**同一已声明端点**上，不新增目标主机或字段。
  - Range 读（`HttpRangeReader::read`）：429/5xx、连接层失败、提前断流、流读错误共用一份预算
    ——每次 `read` 最多 3 次重试，等待 100ms 起指数递增、单次封顶 1s（最坏总等待约 0.7s + 请求耗时）；
    429/503 带 `Retry-After` 时优先采纳，封顶 5s。预算耗尽才向上报错。
  - 整文件下载（`download_to_file`）：仅对「尚未开始传字节」的失败重试（可重试状态码与连接层错误），
    最多 3 次，等待 500ms 起指数递增、封顶 8s，`Retry-After` 封顶 15s；响应体半路断开**不**在此重连
    （无 Range 续传），由 `AudioCache` 的字节数校验丢弃临时文件。
  - 队列自动切歌（`queue_watcher_loop`）：底层播放失败不再以 250ms 循环节奏重放同一首——
    退避 1s 起指数递增、封顶 15s，且**只在连续失败的第一次**发 `playback-error` 事件，
    恢复正常播放即清零。
  - 退避计算集中在 `services/backoff.rs`（纯函数，含单测），其余模块不再各自散落 sleep 常数。
- **未纳入退避的**：`probe_connection` / PROPFIND / MKCOL / DELETE / PUT 快照 / 元数据与歌词侧请求仍为
  单次尝试（失败即按各自口径提示）。歌词侧「每次切歌可能重发」的负缓存缺失依旧存在，登记在 §2A 与 I4 候选。
- **降级**：`root_uri` 为空时连接探测退化到 `http://localhost`（`probe_connection`），属本机探针行为，不外发用户数据。
- **TLS**：使用 rustls + `danger` 之外的默认校验，全仓无 `danger_accept_invalid_certs`，因此自签证书会连接失败（不做静默放行）。服务端差异与降级口径见 I3 的 `WEBDAV_COMPATIBILITY.md`。

### F. 数据备份恢复（对外称谓，代码模块名仍为 `sync`）

- **上传（一次点击的请求序列）**：`services/sync.rs::sync_upload`
  1. `VACUUM INTO` 本地临时快照 → 清空凭据 → `PRAGMA integrity_check`（任一步失败即中止，不发请求）；
  2. `MKCOL` 确保远程目录存在（已存在返回 405，视为成功）；
  3. `PUT <remote_path>/lumo.sqlite.tmp-<pid>-<nanos>`：完整库快照（不含凭据）；
  4. `PUT <remote_path>/lumo.sqlite.sha256`：上一步文件的 SHA-256（64 位十六进制纯文本）；
  5. `MOVE` 临时名 → `lumo.sqlite`（原子替换正式名）。服务器不支持 MOVE 时退回
     `DELETE` 临时名 + 直接 `PUT lumo.sqlite`，并 `tracing::warn!` 留痕。
- **恢复**：`GET lumo.sqlite` → 落**唯一名**本地临时文件（`lumo.sqlite.<pid>-<nanos>.download`，
  并在开始前清理 1 小时前的陈旧残留）→ `GET lumo.sqlite.sha256` 比对（无 sidecar 则跳过）→
  迁移前甄别（SQLite 文件头、大小、schema 版本 ∈ [V8, 本机版本]）→ 迁移 →
  `integrity_check` + `foreign_key_check` + 必要表 → 才 `Backup::restore` 进 live 库。
  **恢复失败时保留回滚副本并把路径告知用户**（I3/G-04）。
- **语义**：整库替换，不做实体级合并、无冲突处理，因此本计划统一称「备份恢复」而非「同步」。
- **远端新增产物**：除 `lumo.sqlite` 外，本版本起还会写入 `lumo.sqlite.sha256`，
  以及 MOVE 失败时可能短暂残留的 `lumo.sqlite.tmp-*`。对外披露时必须算作两个文件。
- **凭据剔除**：快照在上传前清空 `sync_config.password_encrypted` 与 `sources.credential_ref`
  ——**远端快照不含任何口令/密钥**；恢复后需重新填写凭据。该剔除现在在事务内完成，
  **失败即中止上传**（此前是 `let _ =`，脱敏失败会把带口令的快照照常发出）。
- **仍包含的数据**：全部曲库元数据、**本地绝对/相对路径**、播放历史、歌单、收藏。跨设备漫游意味着这些数据上传到用户自己的 WebDAV，须在 UI 明确告知（§4 R-05）。
- **门禁**：上传/恢复受 `config.enabled` 拦截（`commands/sync.rs`），前端默认 `enabled=false`（`src/stores/sync.ts`）。
- **缺陷（P2，未修）**：打开设置页即自动执行 `checkRemote()`（PROPFIND，`Settings.vue`），并非用户显式动作，与「用户授权联网」原则相冲突 → I2。
- **缺陷（P1，未修）**：桌面端「从云端恢复」**没有二次确认**（移动端要求键入「恢复」二字），
  误点即整库替换 → I3 恢复 UI 重做（R-05 同源）。

### G. GitHub 更新检查

- **入口**：`https://api.github.com/repos/huanghao-9708/lumo/releases/latest`（`src/components/mobile/MobileSettings.vue:147`），成功后 `window.open` 跳转发布页（`:172`）。
- **仅移动端、仅手动点击按钮**（`:610`）触发；桌面「关于」页只读本地版本（`Settings.vue:469-484`）。
- **无自动检查、无 tauri updater 插件**（`Cargo.toml`、`tauri.conf.json` 均无 updater endpoints），即应用不会自行下载或安装更新。
- 本项未在设置页声明，属轻微缺口（§4 R-06）。

## 3. 连接层事实

| 项 | 现状 | 证据 |
|---|---|---|
| TLS 后端 | rustls 0.23 + `ring`（纯 Rust，为 Android 交叉编译所选），进程内安装默认 provider | `Cargo.toml` rustls 依赖项、`lib.rs:248-253` |
| 证书校验 | 全程开启，无跳过开关 | 全仓无 `danger_accept_invalid_certs` |
| 系统代理 | reqwest 启用 `system-proxy` feature，**所有外联可能经由系统代理/PAC** | `Cargo.toml` reqwest features；全仓未调用 `no_proxy()` |
| User-Agent | 三套并存：封面用浏览器伪装 UA、歌词用 `LumoMusicPlayer/1.0.0`、WebDAV/AI/GitHub 用 reqwest 或 WebView 默认 UA | `cover.rs:16`、`commands/library.rs:421` |
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

- 日志初始化：`tracing_subscriber::fmt::init()`（`lib.rs:246`），**仅 stdout / logcat，无文件 appender**；IPC 追踪 `ipc_trace.rs` 只记命令名与耗时，不落参数。
- **已确认的泄露点**：
  1. `services/webdav.rs` 的 `HttpRangeReader::read` 用 `tracing::error!/warn!` 打印完整请求 URL；
     若用户把账号写进 URL 的 userinfo 段，口令会进日志。
  2. `WebdavClient::describe_reqwest_error` 把含 URL 的错误原文透传到前端 toast，等于把上述内容显示给用户并可被截图传播。
  3. `services/ai.rs` 记录服务商响应前 200 字符（`text.chars().take(200)`）。
- 未发现明文 API Key 或口令进入日志的路径（Key 仅在请求头，`bearer_auth`）。日志脱敏与文件落盘方案归 I5（诊断包）一并处理。

## 6. 变更控制规则

1. 新增出网必须随 PR 提交本文件的更新，并在 §1 表格占一行；评审时缺此项即打回。
   CI 会用 `scripts/check-network-registry.mjs` 机器校验：Rust 源码里任何以字符串字面量
   书写的 `http(s)://` 目标，其所在文件（相对 `src-tauri/src` 的路径，如
   `commands/library.rs`）必须出现在本文件中。**这条规则不依赖 reviewer 记性**。
2. 新增外联默认关闭；任何「应用启动即请求」的设计一律视为 P0 阻断。
3. 本文件与隐私政策、设置页文案不一致时，**以代码为准并立即修正文档**，不得为了维护旧承诺而隐瞒真实行为。
4. 域名清单中「由服务端响应决定的动态主机」（网易云 CDN、mzstatic）在对外政策里以「及其内容分发网络」表述，不假装可枚举。

## 7. 待人工复核

- reqwest 0.13 的 `system-proxy` 在 Windows 上是否读取注册表并执行 PAC（影响 R-09 的披露粒度）。
- §2A 中「本地纯文本歌词是否应视为已命中」的产品判定。
