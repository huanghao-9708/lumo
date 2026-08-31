# MA0：移动端技术验证与工程基线（Spike）

> 状态：未开始　|　预估：P50 4d / P80 6d　|　前置依赖：无（移动端第一站）
> 配套总体计划：[00_移动端总体迭代计划.md](./00_移动端总体迭代计划.md)

## 1. 目标与背景

用最短时间把移动端的四大不确定性变成**明确的 Go/No-Go 结论**，为后续五个迭代扫清未知数：

1. **交叉编译**：现有 Rust 代码能否为 `aarch64-linux-android` 编译（预期在 reqwest 的 OpenSSL 依赖处失败，需切 rustls）。
2. **音频输出**：rodio → cpal → oboe 链路在 Android 真机上能否出声（本项目最大技术风险 R1）。
3. **网络**：reqwest（rustls）在真机上能否完成 WebDAV HTTPS 请求。
4. **WebView**：现有移动端 UI（M1-M6）与 `lumo://artwork` 自定义协议在 Android WebView 上是否正常。

**本迭代交付的是结论和基线，不是功能**。结束时应产出：一台能跑 Lumo 移动 UI、能出声、能连 WebDAV 的真机 + 一份 ADR 决策记录 + 一套可重复的构建命令。

## 2. 范围

**范围内**：工具链搭建、Cargo 依赖调整（TLS）、debug 构建真机运行、音频/网络/渲染三项 Spike、gen 目录版本管理决策落地。

**范围外**：任何权限申请（MA1）、后台播放（MA2）、UI 打磨（MA4）、签名发布（MA5）。Spike 用的音频测试命令在 MA1 结束时移除或转入 debug-only 入口。

## 3. 任务分解

### A0-1 Android 工具链与交叉编译基线（0.5d）

**环境清单**（用户已备好，此处逐项核对并记录版本）：

```bash
rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android
cargo install cargo-ndk     # 交叉编译封装，自动注入 NDK 环境变量
# 环境变量：ANDROID_HOME、NDK_HOME（指向具体 NDK 版本目录）
# JDK：17+（Tauri 2 Android 要求）；真机：开发者模式 + USB 调试已开启
java -version && rustc -V && cargo ndk --version   # 记录输出到执行记录
```

**验证**：对当前工作区直接交叉编译（预期失败，失败点即信息）：

```bash
cargo ndk -t arm64-v8a -p 24 check
```

预期在 `openssl-sys`（reqwest 默认 native-tls 拉入）处报错。**记录完整错误摘要**——这是 ADR-1 的直接证据。

**验收**：命令执行成功或失败点被明确记录；工具链版本写入执行记录。

### A0-2 reqwest 切换 rustls（0.5d）

修改 `src-tauri/Cargo.toml`：

```toml
# 原：reqwest = { version = "0.13.4", features = ["blocking", "json", "stream"] }
reqwest = { version = "0.13.4", default-features = false, features = [
  "blocking", "json", "stream", "rustls-tls-native-roots",
] }
```

- 选 `rustls-tls-native-roots`（而非 webpki-roots）：保留操作系统证书库，兼容企业内网 CA，与 Windows schannel 行为最接近。
- **全平台统一**切换（不按 target 分叉依赖），桌面端同步回归，避免两套 TLS 栈并存。

**桌面回归**（必须全绿才能合入）：

```bash
cargo check --all-targets && npm run build
# 手工：连接现有 WebDAV 源浏览目录 + 流播一首（验证证书链校验行为无变化）
```

**验收**：`cargo ndk -t arm64-v8a -p 24 check` 通过；Windows 桌面 WebDAV 功能正常。若用户真实 WebDAV 为自签名证书且握手失败，按 R4 预案记录（本迭代不实现 workaround，仅记录）。

### A0-3 真机 Dev 运行与 WebView 渲染验证（1d）

```bash
npm run tauri android dev        # USB 连接真机；Wi-Fi 调试可设 TAURI_DEV_HOST=<手机IP>
```

**验证点清单**：

| # | 验证点 | 通过标准 | 常见问题与处置 |
|---|---|---|---|
| 1 | 应用安装并启动，进入 MobileLayout | 显示 TabBar + 4 Tab 底部导航，非桌面五区布局 | 若显示桌面布局：视口宽度 ≥768（平板/横向）或 WebView 初始宽度异常，记录现象（正式修复在 MA1-2，此处仅确认手机竖屏正常） |
| 2 | 暗色模式跟随系统 | 系统切深色 → UI 切换 | M6 已实现 race 修复，验证真机行为 |
| 3 | 列表滚动 | MobileContentView 各 tab 可切换、滚动不崩溃 | 白屏/JS 报错 → `adb logcat -s chromium` 查看控制台 |
| 4 | `lumo://artwork` 自定义协议 | 任意有封面的视图封面可显示（无数据库时可跳过，MA1 再验） | wry Android 通过 `shouldInterceptRequest` 拦截，理论支持；若失败是 G9 升级为阻断项 |
| 5 | DevTools | `chrome://inspect` 能附加 WebView，可看 console | 用于后续所有前端调试 |

**验收**：手机竖屏下 1/2/3/5 全通过；4 在有数据时通过（允许顺延到 MA1 复验）。

### A0-4 音频输出 Spike（1～1.5d，最高优先）

**做法**：在 `src-tauri/src/commands/playback.rs` 增加临时命令（`#[cfg(debug_assertions)]` 修饰，MA1 收尾移除）：

```rust
#[tauri::command]
#[cfg(debug_assertions)]
pub fn debug_play_tone(app: tauri::AppHandle) -> Result<(), String> {
    // 1) 走与正式链路完全相同的初始化路径：
    //    PlaybackManager::new() -> OutputStream::try_default() -> Sink::try_new
    // 2) 播放 440Hz 正弦 3 秒（rodio::source::SineWave）
}
```

前端在 MobileSettings 加一个 debug 入口按钮触发（或 dev 构建启动 3 秒后自动触发一次）。

**已知风险与探针**（R1 的具体化，逐项记录结论）：

| 探针 | 现象 | 处置 |
|---|---|---|
| P-1 编译 | cpal/oboe 链接失败 | 记录错误；尝试 cpal 指定 oboe feature / 升级 rodio-cpal 组合 |
| P-2 初始化 panic/异常 | `OutputStream::try_default()` 报 JNI/线程错误 | 改在主线程初始化：`app.run_on_main_thread(...)` 后再试；记录可行路径（影响 MA2 的初始化位置设计） |
| P-3 无声但无报错 | oboe 流创建成功但听不到 | 检查音量/音频焦点；尝试 cpal 默认设备枚举；记录设备型号 |
| P-4 变速/变调 | 设备默认采样率（常见 48kHz）与音源（44.1kHz）不匹配 | 记录；MA1 播放真实文件时复测，必要时在 append 前做采样率适配 |
| P-5 后台静音 | 切后台立刻无声 | 预期内（无前台服务），记录现象即结论，MA2 解决 |

**验收**：真机扬声器/耳机听到正弦音 → ADR-2 = Go。任一探针无法绕过 → 按总体计划 R1 预案评估 ExoPlayer 降级方案，**升级为与用户对齐的决策点**。

### A0-5 WebDAV HTTPS 连通性 Spike（0.5d）

在 debug 入口增加第二个临时命令：对用户真实 WebDAV 服务器发一次 `PROPFIND`（复用 `src-tauri/src/services/webdav.rs` 现有 client），返回目录数与耗时。

**验收**：真机上 TLS 握手成功、能列出目录。http:// 明文源在此阶段记录现象（cleartext 由 Manifest `usesCleartextTraffic` 控制，MA3 统一处理）。

### A0-6 gen/android 版本管理落地（0.5d）

1. 确认根 `.gitignore` 未忽略 `src-tauri/gen/android`（现状：只忽略了 `gen/schemas`，符合需要）。
2. `git add src-tauri/gen/android`（其内部 .gitignore 已排除 `build/`、`.gradle`、`local.properties`、`key.properties` 等）。
3. 在 `package.json` 锁定 `@tauri-apps/cli` 精确版本（`"2.x.y"` 无 `^`），与 gen 模板版本绑定。
4. 在 gen 内所有后续手写位置统一加 `// LUMO-CUSTOM: <原因>` 注释标记（本迭代可能还没有手写点，规则先立好）。

**验收**：`git status` 干净且 gen/android 已入库；CLI 版本锁定；提交信息建议 `chore(android): 纳入 gen/android 工程并锁定 tauri cli 版本`。

### A0-7 Spike 结论落档（0.5d）

在本文档「执行记录」追加：每个探针的实际结果、设备信息（型号/Android 版本/WebView 版本）、ADR-1/ADR-2 的最终结论、遇到的坑与解法。同步更新总体计划 ADR 表状态列。

## 4. 测试计划

本迭代无自动化测试要求（均为真机手工 Spike），但 A0-2 的桌面回归三件套必须执行并记录输出。

## 5. 验收清单（迭代退出条件）

- [ ] `cargo ndk -t arm64-v8a -p 24 check` 通过（rustls 生效）
- [ ] `npm run tauri android dev` 在真机安装运行，手机竖屏显示移动端布局
- [ ] debug 正弦音在真机可闻（ADR-2 = Go，或已升级为明确的降级决策）
- [ ] 真机 WebDAV HTTPS PROPFIND 成功
- [ ] `chrome://inspect` 可附加调试
- [ ] gen/android 入库，CLI 版本锁定
- [ ] Windows：`cargo check --all-targets` + `npm run build` + 手工 WebDAV 回归通过
- [ ] 本文档执行记录与总体计划 ADR 状态已更新

## 6. 风险与回退

- **最大风险**：A0-4 音频失败（R1）。回退路径：ExoPlayer 插件方案（解码/输出移至 Kotlin，Rust 保留曲库/队列/IPC），预估 +8 工程日，需重新评审 MA1/MA2 的任务结构。
- **次级风险**：rustls 与用户服务端不兼容（R4）。回退路径：显式「跳过证书校验」设置项（默认关），或该迭代内先以 http 源继续。
- **回退整体**：本迭代所有 Rust 改动集中在 Cargo.toml 一处（TLS）+ debug-only 临时命令，随时可整体撤销，不影响桌面主线。

## 7. 执行记录

> 迭代执行时按日追加：日期 / 做了什么 / 实际与计划的偏差 / 遗留问题。

（待填写）
