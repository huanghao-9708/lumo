# Lumo (轻音) 本地音乐播放器

Lumo 是一款本地优先、零自建服务端、跨平台的高品质桌面音乐播放器。它基于本地 SQLite 建立统一曲库索引，支持快速扫描本地音频目录并解析歌曲标签，并在后续阶段逐步整合 WebDAV 远程曲库。

---

## 🚀 项目特点

- **本地优先**：曲库浏览、检索、歌单、收藏、历史全部读写本地 SQLite，秒级响应，离线可用。
- **默认离线**：在线歌词、在线封面、AI 电台、数据备份**默认全部关闭**，需逐项手动开启；无任何遥测、埋点或崩溃上报。联网行为逐条列在 [联网行为清单](./resources/doc/product/NETWORK_BEHAVIOR.md)。
- **文件与歌曲分离**：同一首歌曲可关联多个文件版本（本地与 NAS 重复、MP3 与 M4A 双版本），配合多音源优先级归并，保证曲库建模长期稳定。
- **轻量高效**：基于 Tauri 2.x，相比 Electron 内存与安装包体积显著更小（具体数值见性能基准文档，不做未经测量的承诺）。
- **非侵入**：只建立索引指向你的音乐文件，不移动、不复制、不修改源文件；卸载后音乐完好。
- **支持的音频格式**：`MP3`、`FLAC`、`WAV`、`M4A`、`AAC`（全平台一致，以扫描器白名单为准）。其他格式即使解码库支持也不入库。

> ⚠️ **平台支持范围**：Windows x64 与 Android (aarch64) 为正式支持平台；**macOS 与 Linux 为技术预览**（CI 构建但未做真实验证）。Android 目前为**预览版**——真机冒烟矩阵尚未执行。详见 [功能与平台矩阵](./resources/doc/product/FEATURE_MATRIX.md)。

---

## 📖 产品与工程文档

| 文档 | 作用 |
|---|---|
| [产品契约 PRODUCT_CHARTER](./resources/doc/product/PRODUCT_CHARTER.md) | 定位、不可妥协原则、术语契约、平台承诺分级、本期冻结范围 |
| [功能与平台矩阵 FEATURE_MATRIX](./resources/doc/product/FEATURE_MATRIX.md) | **唯一功能状态事实源**：每项能力在各平台的状态与验证证据 |
| [联网行为清单 NETWORK_BEHAVIOR](./resources/doc/product/NETWORK_BEHAVIOR.md) | 所有外部请求的目标、触发条件、发送字段与关闭方式 |
| [版本与发布政策 VERSION_POLICY](./resources/doc/release/VERSION_POLICY.md) | 发布通道、SemVer 规则、发布门禁、支持窗口、热修复条件 |
| [决策日志 DECISION_LOG](./resources/doc/product/DECISION_LOG.md) | 关键取舍及其证据 |
| [Post-Stable Backlog](./resources/doc/product/POST_STABLE_BACKLOG.md) | 成熟化期间冻结的需求 |
| [商业成熟化总计划](./resources/doc/commercial-maturity/README.md) | I0–I6 迭代路线与发布门禁 |
| [移动端迭代文档](./resources/doc/mobile/README.md) | MA0–MA5 执行记录 |

## 🛠️ 技术选型

### 前端渲染层
* **核心框架**：Vue 3 (TypeScript)
* **样式构建**：Tailwind CSS v4 (基于原生 CSS 变量的现代化主题驱动)
* **构建工具**：Vite

### 后端核心层 (Rust)
* **桌面容器**：Tauri 2.x
* **数据存储**：SQLite (`rusqlite` 带 `bundled` 特性)
* **标签解析**：`lofty` (读取 APE/FLAC/MP3 等标签与封面)
* **音频解码**：`symphonia` (纯 Rust 编写的优秀音频解码库)
* **音频输出**：`rodio` (管理底层音频输出流)

---

## 📂 目录结构说明

```text
lumo/
├── src-tauri/                # Rust 核心引擎（Tauri 2.x）
│   ├── src/
│   │   ├── main.rs           # 二进制入口
│   │   ├── lib.rs            # 应用装配：状态、插件、托盘外生命周期、lumo:// 协议
│   │   ├── db.rs             # 连接池（WAL）+ 版本化迁移
│   │   ├── models.rs         # DTO 定义
│   │   ├── error.rs          # AppError（thiserror，可直接跨 IPC 序列化）
│   │   ├── ipc_trace.rs      # IPC 耗时探针
│   │   ├── commands/         # IPC 命令层：library / playback / queue / scanner / sync / ai / app / debug
│   │   ├── services/         # 领域服务：scanner / playback / queue / webdav / cache / cover / metadata / sync / secret / ai / platform / file_priority / library
│   │   └── repositories/     # SQL 层：track / album / artist / playlist
│   ├── capabilities/         # Tauri 权限清单
│   └── gen/android/          # Android 工程（MainActivity + MediaPlaybackService 前台服务）
├── src/                      # Vue 3 前端
│   ├── api/                  # IPC 调用封装（按领域分文件）
│   ├── stores/               # Pinia：player / ui / sync / ai
│   ├── composables/          # usePlatform / useVirtualList / useArtworkSrc 等
│   ├── components/
│   │   ├── layout/           # 桌面：TopBar / SidebarLeft / SidebarRight / MainContent / BottomPlayer / NowPlayingImmersive
│   │   ├── content/          # 专辑 / 艺人 / 歌单 / 智能歌单 / 收藏 / 文件夹 / 搜索 / 设置 / AI
│   │   ├── mobile/           # 移动端整套布局与视图
│   │   └── shared/           # 弹窗 / Toast / 歌词 / 批量操作条 / WebDAV 文件夹选择器
│   └── config/appInfo.ts     # 版本号等元信息唯一出口（取自 package.json）
├── resources/doc/            # 产品、工程与迭代文档
└── resources/ui/             # UI 原型与 LDL 设计规范
```

---

## ⚙️ 开发环境配置与启动教程

### 1. 前置依赖准备

在您的系统上运行 Lumo 之前，需要确保已安装以下开发工具：

#### 基础工具：
- **Node.js** (推荐 v18 或更高版本)
- **Rust 工具链** (安装 `rustup`，默认使用 `stable-x86_64-pc-windows-msvc`)

#### Windows 编译环境（MSVC 与 SDK）：
由于 Rust 底层依赖需要编译 C-bindings，且需要调用 Windows 原生 API，**必须安装 C++ 编译环境**：
1. 下载并打开 [Visual Studio Installer](https://visualstudio.microsoft.com/zh-hans/visual-cpp-build-tools/)。
2. 安装或修改 **Visual Studio 生成工具** (Build Tools 2022)。
3. 在“工作负荷”中勾选 **使用 C++ 的桌面开发**。
4. 确保在右侧组件树中勾选了以下两项：
   - **MSVC v143 - VS 2022 C++ x64/x86 生成工具**
   - **Windows 11 SDK** 或 **Windows 10 SDK**。
5. 点击修改/安装，等待安装完成。

---

### 2. 启动开发模式

1. **克隆项目并进入根目录**：
   ```powershell
   cd d:\code\rust\lumo
   ```

2. **安装 Node 依赖包**：
   ```powershell
   npm install
   ```

3. **运行开发服务器（会自动拉起 Tauri 桌面窗口）**：
   ```powershell
   npm run tauri dev
   ```
   > 💡 **提示**：如果您的终端环境未自动加载 Visual Studio 环境变量，请在**开发者命令提示符 (Developer PowerShell for VS 2022)** 下执行此命令，或在命令前载入 `vcvars64.bat`：
   > ```powershell
   > cmd /c "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat && npm run tauri dev"
   > ```

---

### 3. 应用打包发布

若要将应用编译并打包成独立的安装程序（如 Windows `.msi` 或 `.exe`），请运行：
```powershell
npm run tauri build
```
编译完成后，安装包将生成在项目目录下的 `src-tauri/target/release/bundle/` 中。

---

### 4. 移动端 (Android) 构建与运行

Lumo 原生支持 Android 移动端，具备原生后台服务保活、通知栏/锁屏五键控制、来电暂停与拔耳机自动暂停、WebDAV 云端流播与透明缓存等能力。

1. **添加 Rust Android 目标架构**：
   ```powershell
   rustup target add aarch64-linux-android
   ```
2. **连接手机或模拟器进行调试**：
   ```powershell
   npm run tauri android dev
   ```
3. **打包 Release 架构 APK**：
   ```powershell
   npm run tauri android build -- --target aarch64 --apk
   ```
   产物 APK 位于：`src-tauri/gen/android/app/build/outputs/apk/release/`。

---

## 📱 平台与移动端状态

MA0–MA5 的**工程实现已全部落地**（代码、构建、CI 签名、能力清单见各迭代文档）。但**验证证据目前只到模拟器**：

- [x] MA0 技术验证与工程基线（Pixel 7 模拟器 / Android 17 / x86_64；**真机未接入**）
- [x] MA1 Android 工程化与本地播放闭环（运行时权限、来源管理、扫描入库、首尾循环、冷启动恢复）
- [x] MA2 后台播放与系统媒体集成（Rust 权威队列下沉、Kotlin 前台服务、音频焦点、拔出耳机暂停）
- [x] MA3 WebDAV 与数据备份恢复（来源管理与能力探测、透明缓存、离线置灰、统一 SecretStore、二次防误触）
- [x] MA4 移动体验打磨（安全区避让、列表虚拟排版、封面横滑切歌、空态引导）
- [x] MA5 打包发布与质量门禁（Release 签名构建、发布检查清单、应用内更新检查、诊断导出）
- [ ] **真机功能冒烟矩阵（10 项）**——见 [发布检查清单](./resources/doc/mobile/发布检查清单.md)，完成前 Android 按**预览版**对待

因此本仓库不使用"同步"描述跨设备能力：当前的 WebDAV 数据库能力是**整库快照备份与覆盖恢复**，不具备实体合并与冲突处理。

