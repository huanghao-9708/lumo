# Lumo (轻音) 本地音乐播放器

Lumo 是一款本地优先、零自建服务端、跨平台的高品质桌面音乐播放器。它基于本地 SQLite 建立统一曲库索引，支持快速扫描本地音频目录并解析歌曲标签，并在后续阶段逐步整合 WebDAV 远程曲库。

---

## 🚀 项目特点

- **本地优先**：曲库浏览、检索、歌单、历史等数据全部存储于本地 SQLite 数据库中，秒级响应，网络离线不影响曲库操作。
- **文件与歌曲分离**：采用先进的去重与合并建模，同一首歌曲可关联多个文件版本（如本地和 NAS 重复、MP3 和 FLAC 双版本），保证数据库架构长期稳定。
- **轻量高效**：基于 Tauri 2.x 构建桌面外壳，相较于 Electron 应用，内存占用更低，安装包体积大幅缩小。
- **极速体验**：前端采用 Vue 3 (TypeScript) 和 Tailwind CSS v4 驱动，界面丝滑并原生支持深浅色双主题无缝切换。

---

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
├── src-tauri/               # Tauri 2.x Rust 后台代码
│   ├── Cargo.toml           # Rust 依赖项与打包配置
│   ├── src/
│   │   ├── main.rs          # 应用入口点
│   │   └── lib.rs           # 核心控制与 Tauri Commands
│   └── tauri.conf.json      # Tauri 应用配置文件
├── src/                     # Vue 3 前端代码
│   ├── assets/              # 前端静态资源
│   ├── components/          # Vue 组件化代码 (Sidebar, PlayerBar, MainContent 等)
│   ├── store/               # 全局 Mock 数据与状态管理层
│   ├── App.vue              # 前端主入口组件
│   ├── main.ts              # Vue 实例化入口
│   └── style.css            # 整合 Tailwind CSS v4 的全局样式表
├── resources/
│   └── ui/                  # UI 原型及参考设计图
├── package.json             # 前端项目依赖与运行脚本
└── README.md                # 本说明文档
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

## 📅 移动端迭代开发状态（MA0 - MA5）

- [x] **MA0：技术验证与工程基线**（移动自适应布局渲染、rodio 纯 Rust 解码输出、WebDAV 连通）
- [x] **MA1：Android 工程化与本地播放闭环**（运行时权限、存储建议、本地扫描入库、首尾循环、冷启动恢复）
- [x] **MA2：后台播放与系统媒体集成**（Rust 权威队列下沉、Kotlin 前台服务、MediaSession、音频焦点抢占、拔耳机自动暂停）
- [x] **MA3：WebDAV 与跨端数据同步**（WebDAV 来源管理与探测、透明缓存、离线置灰、统一 SecretStore 凭据安全、二次防误触云端恢复）
- [x] **MA4：移动体验打磨与性能优化**（安全区避让、列表虚拟排版、封面横滑切歌、空态引导、本地化名称）
- [x] **MA5：打包发布与质量门禁**（Release 签名构建体系、发布检查清单、应用内更新检查、诊断导出）
