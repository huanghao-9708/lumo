# WebDAV 兼容性矩阵与已知限制（I3/DATA-003）

> 状态：v1，2026-09-18  
> 事实来源：`src-tauri/src/services/webdav.rs`、`src/commands/playback.rs`、`src/commands/scanner.rs`、
> `src/services/scanner.rs`、`src/services/cache.rs`、`src/services/sync.rs`（逐行读码核实，非推测）。  
> 口径：**本文件区分「代码行为」与「实测证据」两件事**。代码行为可以现在就写死；
> 实测状态列在没有真机/真服证据前一律标 ❔，且**不得**据此对外声明"兼容某类服务器"
> （`FEATURE_MATRIX.md` §4 规则 7）。

## 1. 客户端协议实现清单

| 能力 | 实现 | 行为要点 |
|---|---|---|
| 认证 | 仅 HTTP Basic（`apply_auth`，两个 client 都加） | 无 Digest / Bearer / 表单登录 / OAuth。凭据解析统一走 `commands/scanner.rs::resolve_source_credential`（钥匙串引用 / V6 加密 / V5 明文读兼容） |
| 连接探测 | `probe_connection`：`PROPFIND Depth: 0` | 返回 `ok / latency_ms / server_header / status_code / error`（`WebdavProbeResult`）；`root_uri` 为空时退化到 `http://localhost` 本机探针 |
| 目录枚举 | `propfind`：`PROPFIND Depth: 1` + 扫描侧自建树（栈式逐层下钻） | **不依赖 `Depth: infinity`**——多数 NAS/网盘代理不支持 infinity，这条路径绕开了它。代价：请求数 ≈ 目录数 |
| XML 解析 | `parse_propfind`，quick-xml 事件流 | 命名空间无关（取 `:` 之后的本地名）：`<d:response>` 与 `<response>` 等价；识别 `href` / `getcontentlength` / `getlastmodified` / `collection`（`Start` 与自闭合 `Empty` 都认，因此 `<resourcetype><collection/></resourcetype>` 和 `<d:collection/>` 都能判定目录）。**不读** `getetag`、`creationdate`、quota 类属性 |
| 解析失败 | 中止扫描（I3/G-03 修复） | `Err(_) => break` 的老写法会把截断响应当完整列表，进而把整个来源误标 missing。现在 PROPFIND 解析失败 → `scan_failed = true` → 不做缺失标记 |
| Range 探测 | `probe_range_support`：真实文件 `GET` + `Range: bytes=0-0` | 206 → 支持；200 → 明确忽略 Range；401/403/404/其他 → `Err`，**不落库**（探测失败不污染缓存的能力位） |
| Range 结果持久化 | 表 `source_capabilities.supports_range`（`checked_at` 记时间） | 每来源一次探测，之后直接读缓存（`commands/playback.rs::load_range_support`）。**没有 TTL / 复测策略**，见 §5 L-2 |
| Range 读 | `HttpRangeReader: Read + Seek` | 每次 `read` 最多 3 次退避重连（I3/G-12）；前向 seek ≤ 256KB 走"跳过读"复用当前响应，超过则重发 Range 请求 |
| Content-Range 校验 | 严格 | 206 响应若 `Content-Range` 起始偏移 ≠ 请求偏移 → 立即 `InvalidData` 报错停止解码（错段会让解码结果静默错乱，比失败更糟） |
| Range 被忽略 | 仅在 `offset == 0` 时接受整条 200 流并 `warn` | 从错误偏移解码的路径一律拒绝；不支持 Range 的来源在探测后改走整文件下载 |
| 整文件下载 | `download_to_file`（`bulk_client`） | 无总超时 + TCP keepalive（空闲 30s / 间隔 10s）探测死连接（I3/G-09）；返回**实际写入字节数**，由 `AudioCache::store_from_webdav` 比对 `media_files.file_size` 后才 `rename`（I3/G-10） |
| 429/5xx | 有限指数退避（I3/G-12） | Range 读：3 次、100ms 起封顶 1s、`Retry-After` 封顶 5s；整文件下载：3 次、500ms 起封顶 8s、`Retry-After` 封顶 15s；预算耗尽报"服务器暂时不可用"，与"不支持分段读取"区分 |
| 文本抓取 | `fetch_text(url, max_bytes)` | 歌词 `.lrc` 之类小文件，带字节上限 |
| 目录创建 | `mkcol` | `405`（已存在）幂等视为成功 |
| 上传 | `put_file`（`bulk_client`） | 备份快照与校验和 sidecar |
| 重命名/替换 | `move_file`（`MOVE`，带 `Destination` 与 `Overwrite: T`） | 快照上传的原子替换：`PUT lumo.sqlite.tmp-<rand>` → `PUT` 校验和 → `MOVE` 覆盖正式名；**MOVE 失败退回直接 `PUT` 覆盖**并 `warn`（`services/sync.rs`），此时唯一防线是恢复侧的文件头 + `integrity_check` 校验 |
| 删除 | `delete` | `204` 与 `404` 都视为成功 |
| TLS | rustls，默认证书校验 | 全仓无 `danger_accept_invalid_certs` / 无自定义 `danger` 配置：自签或证书链不完整的服务器**连接失败**，不做静默放行，也**不引导用户关闭校验** |
| URL 拼接 | `build_url`：`Url::join` | `base_url` 来自用户输入、`subpath` 来自远端 XML；非法输入返回错误文案而不是 panic（历史上有 unwrap 链） |

## 2. 服务端差异 → 客户端行为矩阵

「客户端行为」列是代码事实；「实测」列在补上真服证据前一律 ❔。

| 服务端差异 | 客户端行为 | 实测 |
|---|---|---|
| 只支持 `Depth: 1`（多数 NAS / 网盘代理） | 正常工作（扫描侧自己递归下钻） | ❔ |
| 支持 `Depth: infinity` 一次性返回全树 | 同样工作（递归会用返回条目里的目录路径继续下钻，可能重复访问） | ❔ |
| 返回 `<response>` 无命名空间前缀 | 正常工作（本地名匹配） | ❔ |
| 目录只标 `getcontenttype: httpd/unix-directory`，不返回 `collection` | **识别不出目录**：该条目按文件处理，被扩展名过滤丢弃 → 其子目录不会被下钻 → 内部文件缺失（风险，见 §5 L-4） | ❔ |
| 不返回 `getcontentlength`（部分网关） | `file_size = 0` → 缓存有效性退化为"非空即有效"（无期望大小可比） | ❔ |
| `getlastmodified` 非 RFC-2822 | `fs_mtime = 0`，增量扫描的"未变即跳过"判定失效 → 每次重扫重解析标签，正确但慢 | ❔ |
| href 返回百分号编码 vs 原始 UTF-8 | `build_url` 两种都能请求；但扫描侧 `source_root` 取 `Url::path()`（编码形态），若与服务端 href 形态不一致，`strip_prefix` 失败，`normalized_path` 退化为含根前缀的路径（见 §5 L-1） | ❔ |
| 不支持 Range（返回 200 全量） | 探测后走整文件下载 + 本地缓存播放 | ❔ |
| 支持 Range 但 `Content-Range` 偏移错 | 立即报错，停止当前解码（不猜、不续） | ❔ |
| 401 / 403 | 分别提示"认证失败：用户名或密码错误 (401)" / "权限不足 (403)"；**不清空曲库** | ❔ |
| 404（文件已删）vs 404（URL 编码不对） | 目前**不区分**：统一"文件不存在 (404)"（未实现项，见 §5 L-5） | ❔ |
| 429 / 5xx | 有限退避重试，预算耗尽报错（§1 对应行） | ❔ |
| 不支持 `MOVE` | 快照上传退回直接 `PUT` 覆盖 + `warn`（原子性丢失，靠恢复侧校验兜底） | ❔ |
| 自签 / 内网证书 | 连接失败，文案为通用网络错误（无专门"证书不受信"文案，未实现明文风险提示，见 §5 L-3） | ❔ |

## 3. 三类服务端实测规程（待人工执行）

计划要求至少三类。每类记录同一组字段，填进 §2 的「实测」列并附证据链接或日志。

| 类别 | 候选 | 要记录的字段 |
|---|---|---|
| A. NAS / 自建 WebDAV（Nextcloud 类） | Nextcloud、群站/极空间/Synology WebDAV Server | `server` 头、Depth 支持、`collection` 表达方式、href 编码形态、Range(206/200)、`MOVE`、大文件（>1GB）下载、中文+空格+`#%` 路径 |
| B. 对象存储 / 网盘代理类 | 阿里云盘 / 夸克 / CloudDrive / rclone WebDAV / Alist | 同上 + 限流行为（429 与 `Retry-After`）、`getcontentlength` 是否缺失、并发下载是否被掐 |
| C. 标准测试服务器 | `baikal`/`serge-z/webdav-server`/CI 内本地 mock | 已知正确答案下的 PROPFIND / Range / 429 / 断流 / 截断 XML，用于回归 |

每类最小验证序列：

1. 添加来源 → 连接探测（记录 `probe_connection` 的 status / server / latency）；
2. 首次全量扫描（含中文与特殊字符目录）→ 核对 `media_files` 行数与远端文件数一致，`sources.last_error` 为空；
3. 拔网/停服务后重扫 → 断言：**旧记录仍在、不产生 missing 批量标记**（`FEATURE_MATRIX` G-03 的现场复验）；
4. 播放 1 首远程曲目 → 观察 `source_capabilities.supports_range` 落库值与实际路径（流播 vs 整文件缓存）；
5. 播放 GB 级 FLAC → 记录是否被超时掐断（G-09 复验）；
6. 备份上传 + 从云端恢复各 1 次 → 记录 `MOVE` 是否可用、恢复后 `integrity_check` 通过；
7. 全程 `tracing` 日志留档（退避重试次数字段可 grep `后第 .* 次重试`）。

## 4. 自动化验证现状

| 已有（本仓测试） | 覆盖 |
|---|---|
| `services/webdav.rs` 内 `parse_content_range_start` / `build_url` 相关测试 | Content-Range 解析、URL 拼接非法输入不 panic |
| `services/backoff.rs` 5 项测试 | 退避序列、溢出防护、可重试状态集合、`Retry-After` 解析与封顶 |
| `services/cache.rs` 6 项测试 | 截断缓存拒绝、命中 touch、下载守卫 RAII、淘汰跳过 `.tmp`、字节数断言 |
| `services/scanner.rs` / `commands/scanner.rs` 层 | 扫描状态写入（`record_scan_result`）与守卫抢占 |

缺口（登记为 QA-004，I4）：**没有 HTTP 层的 WebDAV mock 测试**。§1/§2 里的重试、断流重连、
200-代替-206、截断 XML 等行为目前只有代码事实与人工验证规程，没有可重复的自动化证据。
纯 `std::net::TcpListener` 起本地假服务即可覆盖，不需要新增依赖。

## 5. 已知限制（对外表述时必须带上）

- **L-1 路径归一化依赖服务端 href 形态一致**：同一来源内若服务端对同类路径给出不同编码形态
  （或代理层改写），`normalized_path` 会出现两套键，表现为重复入库或增量判定失效。需 A/B 类实测确认。
- **L-2 Range 能力位无 TTL**：`source_capabilities` 一旦写入就长期复用。服务端更换/升级导致
  Range 能力变化时，客户端不会自动复测（探测请求本身成本极低，属可优化项，未列入 I3）。
- **L-3 明文 HTTP 与证书异常缺用户提示**：`http://` 来源与自签失败目前只有通用网络错误文案，
  没有"明文传输、凭据可被窃听"的显式风险提示（DATA-003 未完成项）。证书校验**不会**被放宽。
- **L-4 目录识别只看 `collection`**：仅以 `getcontenttype` 标目录的服务端会让子目录漏扫（§2 对应行）。
- **L-5 404 不区分"文件删除"与"URL 编码错误"**：两者都提示"文件不存在"，用户难以自诊。
- **L-6 缓存失效只看大小**：远端同名同字节数被替换时，旧缓存仍判有效（不比对 ETag/mtime）。
- **L-7 无并发/带宽画像**：所有下载与扫描都是单连接顺序；`source_capabilities.max_parallel_requests`
  字段存在但无实现，未纳入任何调度。

## 6. 与发布的关系

- 本文件是 `FEATURE_MATRIX.md` 中 WebDAV 行的证据入口之一；§2/§3 的 ❔ 未清之前，
  对外文案只能写「支持标准 WebDAV（HTTP Basic + PROPFIND + Range），兼容面仍在实测」。
- I6 Beta 准入要求 A、B 两类各有至少一次完整 §3 序列留档。
