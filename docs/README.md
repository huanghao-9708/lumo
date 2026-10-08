# Lumo（轻音）技术与设计全景文档导航

> **项目名称**: Lumo（轻音）  
> **定位**: 本地优先、注重隐私、轻量级跨平台音乐播放器  
> **当前版本**: 2.1.1 (单一事实源: [package.json](file:///c:/Users/hao/RustroverProjects/lumo/package.json))  
> **支持平台**: Windows x64 (正式版候选) / Android aarch64 (预览版) / macOS & Linux (技术预览)

---

## 一、核心文档矩阵

本项目技术文档库分为三大核心领域文档以及已有产品/设计资源索引：

```
lumo/docs/
├── README.md                              # 本文档（全景索引与架构导航）
├── design-language.md                     # 📐 LDL 设计语言规范与实现文档
├── frontend-design.md                     # 🖥️ 前端架构与组件设计文档
└── frontend-backend-integration.md        # 🔗 前后端交互与全栈架构设计文档
```

### 1. [📐 设计语言规范与实现文档 (LDL)](file:///c:/Users/hao/RustroverProjects/lumo/docs/design-language.md)
全面阐述项目的视觉体系、设计基因与实现标准：
- **设计哲学**: Warm Industrial Minimalism（温暖工业极简主义），“Less Interface, More Music”；
- **设计宪法**: 严格恪守十大原则（Content First, Typography Before Decoration, Whitespace Creates Rhythm, Divider Over Card, One Accent Rule, Quiet Controls, Mono for Numbers, Touch Target Rule, Code Follows Design, Accessible by Default）；
- **空间布局**: 1200×720 基线，五区固定工作台（左侧栏 220px、顶栏 60px、内容区、Inspector 320px、底栏 92px）；
- **色彩体系**: 严禁冷灰原则（Strictly No Cold Gray），亮色米白（`#F7F5F1` / `#FBFAF7`）与暗色暖炭黑（`#1C1A17` / `#23211D`），唯一强调色暖橘（`--brand-orange: #E28A23`）；
- **Design Tokens**: 11 档字号阶梯、空间尺寸常量、无阴影原则与 5 个受控阴影例外、Auto-Hide 滚动条机制；
- **组件规范**: 按钮系统、歌曲行（TrackRow 40px/56px、四态高亮、2px 橙色竖条）、专辑卡片、实体质感音量旋钮。

### 2. [🖥️ 前端架构与组件设计文档](file:///c:/Users/hao/RustroverProjects/lumo/docs/frontend-design.md)
深入剖析 Vue 3 前端工程分层、状态管理与交互实现：
- **工程全景**: Vue 3.5 + TypeScript 5.6 + Pinia 3.0 + Tailwind CSS v4 + Vite 6；
- **自研 UI 组件体系**: 绝不依赖任何臃肿第三方 UI 库，纯手工打造 LDL 现代化轻量组件；
- **三态布局系统**: 桌面五区工作台、桌面极简/迷你播放栏（MiniPlayerBar 560×96）、移动端触控骨架（MobileLayout）；
- **状态驱动路由**: 摒弃 `vue-router`，选用基于 Pinia 的响应式 Tab 与历史栈机制（`playerStore.activeLibraryTab` + `historyStack` + 滚动位置记忆还原）；
- **解耦歌曲列表包**: `TrackRow`, `TrackListHeader`, `columns.ts`, `useTrackColumns.ts` 响应式列宽与自适应列收纳规则；
- **Pinia 状态机**: 5 大 Store（`usePlayerStore`、`useUiStore`、`useDesktopModeStore`、`useSyncStore`、`useAiStore`）；
- **9 大 Composables**: 虚拟滚动、批量选择、Canvas 封面主色提取、滚动条监听、Android 实体返回键等；
- **性能与攻坚实践**: WebView2 禁用平滑滚动提速 69 倍、Base64 缩略图内联消除 N+1 冲击、AOP 耗时探针。

### 3. [🔗 前后端交互与全栈架构设计文档](file:///c:/Users/hao/RustroverProjects/lumo/docs/frontend-backend-integration.md)
深度解析 Tauri 2 跨进程通信、Rust 核心引擎与底层音频流水线：
- **IPC 架构与拓扑**: JSON-RPC 请求响应、双向 Events 广播、`lumo://artwork` 自定义协议与 4 并发信号量限流；
- **80+ 命令全量索引**: 9 大模块（曲库管理、底层播放、权威队列、扫描器、备份恢复、AI推荐、桌面偏好、移动桥接、调试）；
- **核心数据模型契约**: TypeScript `types.ts` 与 Rust `models.rs` 1:1 双向映射，实体分离核心哲学（“文件不是歌曲”，`tracks` vs `media_files`）；
- **SQLite 存储引擎**: 23 张核心表、WAL 模式、r2d2 连接池、内存治理页缓存控制、V1~V13 增量幂等迁移链路（含多艺人智能拆分算法）；
- **底层音频流水线**: Symphonia 解码 + Rodio 输出，**锁外解码 + 锁内微秒换源**，变速播放位置矫正；
- **权威播放队列下沉**: Rust 权威状态机保证 Android 锁屏切歌保活，世代号防抖，看门狗 250ms 轮询与 Gapless 3秒无缝预加载；
- **WebDAV 与缓存**: HTTP Range 分段流式播放，`AudioCache` 近似 LRU 本地透明分块缓存；
- **系统凭据与安全隐私**: Windows Credential Manager / macOS Keychain 钥匙串集成，Android 设备熵绑定加密，全网行为白名单门禁与 CI 校验；
- **Android 原生桥接**: Kotlin 前台服务 `MediaPlaybackService.kt`、通知栏五键控制、音频焦点与耳机拔出防误放广播。

---

## 二、项目已有上游规范与工程文档索引

项目在历史演进过程中积累了极为扎实的产品和工程规范，均可作为重要背景资料交叉查阅：

| 目录/文件 | 类别 | 说明 |
|---|---|---|
| [resources/ui/lumo_design/v2/](file:///c:/Users/hao/RustroverProjects/lumo/resources/ui/lumo_design/v2/README.md) | **设计系统** | LDL v2.0 完整设计规范库（包含 10 篇基础规范、10 篇桌面组件规范、8 篇移动组件规范） |
| [resources/doc/product/PRODUCT_CHARTER.md](file:///c:/Users/hao/RustroverProjects/lumo/resources/doc/product/PRODUCT_CHARTER.md) | **产品规范** | 产品宪章（上位法定契约，定义核心原则与非目标） |
| [resources/doc/product/FEATURE_MATRIX.md](file:///c:/Users/hao/RustroverProjects/lumo/resources/doc/product/FEATURE_MATRIX.md) | **产品规范** | 功能矩阵与平台支持唯一事实源（MP3/FLAC/WAV/M4A/AAC 5种白名单格式） |
| [resources/doc/product/NETWORK_BEHAVIOR.md](file:///c:/Users/hao/RustroverProjects/lumo/resources/doc/product/NETWORK_BEHAVIOR.md) | **安全隐私** | 穷尽联网行为清单（代码调用点与 CI 门禁基准） |
| [resources/doc/product/DECISION_LOG.md](file:///c:/Users/hao/RustroverProjects/lumo/resources/doc/product/DECISION_LOG.md) | **架构决策** | 架构与产品取舍决策日志（D-01 至 D-08） |
| [resources/doc/release/VERSION_POLICY.md](file:///c:/Users/hao/RustroverProjects/lumo/resources/doc/release/VERSION_POLICY.md) | **发布工程** | 版本号单一事实源与发布通道政策 |
| [resources/doc/desktop-modes/](file:///c:/Users/hao/RustroverProjects/lumo/resources/doc/desktop-modes/README.md) | **专项技术** | 桌面显示模式与迷你播放栏 M0~M5 重构设计与执行记录 |
| [resources/doc/commercial-maturity/](file:///c:/Users/hao/RustroverProjects/lumo/resources/doc/commercial-maturity/README.md) | **专项技术** | 商业成熟化计划（I0~I6 质量、安全、WebDAV 兼容性与发布演练） |
| [resources/doc/mobile/](file:///c:/Users/hao/RustroverProjects/lumo/resources/doc/mobile/README.md) | **专项技术** | 移动端 Android MA0~MA5 完整工程闭环设计文档 |
| [README.md](file:///c:/Users/hao/RustroverProjects/lumo/README.md) | **仓库主说明** | 项目概览、架构说明与本地编译调试启动指南 |

---

## 三、工程技术栈总览表

```
┌─────────────────┬──────────────────────────────────────────────────────┐
│ 维度            │ 选型与规格                                           │
├─────────────────┼──────────────────────────────────────────────────────┤
│ 前端框架        │ Vue 3.5.13 (Composition API, <script setup lang="ts">)│
│ 编程语言        │ TypeScript 5.6.2 (严格模式) / Rust 2021 Edition      │
│ 容器技术        │ Tauri 2.x (@tauri-apps/api 2.x, tauri-cli 2.11)      │
│ 构建工具        │ Vite 6.0.3 (@vitejs/plugin-vue, @tailwindcss/vite)   │
│ 样式与设计语言  │ Tailwind CSS v4.3.0 + 自研 LDL v2.0 规范             │
│ 图标库          │ lucide-vue-next (0.577.0, 1.5px 极简线框风格)        │
│ 状态管理        │ Pinia 3.0.1 (去中心化分模块管理)                     │
│ 嵌入式数据库    │ SQLite 3 (rusqlite 0.32 bundled, r2d2 连接池, WAL)   │
│ 底层音频解码    │ Symphonia 0.5.4 (支持 MP3, FLAC, WAV, M4A, AAC)      │
│ 音频输出混音    │ Rodio 0.21 (纯 Rust 混音与变速支持)                 │
│ 标签元数据解析  │ Lofty 0.21 (ID3v2, Vorbis, APE, MP4 Tags)            │
│ 远程协议与网络  │ reqwest 0.13 (rustls/ring 纯 Rust 后端) / WebDAV     │
│ 系统安全凭据    │ keyring 3.0 (Windows Credential Manager / Keychain)   │
│ 移动原生集成    │ Android NDK, Kotlin (MediaPlaybackService 前台保活)  │
│ 自动化测试      │ Vitest 4.1.11 (happy-dom), cargo test               │
│ 质量与规范门禁  │ scripts/check-versions.mjs, check-network-registry.mjs│
└─────────────────┴──────────────────────────────────────────────────────┘
```

---

## 四、本地开发与快速指引

### 1. 环境准备
- Node.js 18+ (推荐 Node 20 / 22)
- Rust 1.78+ (带 cargo 工具链)
- Windows: Visual Studio 2022 C++ 生成工具 + Windows 10/11 SDK

### 2. 启动与构建常用命令
```bash
# 安装前端依赖
npm install

# 运行全链路 CI 质量门禁 (版本一致性 + 联网行为检查 + TS 编译 + Vitest 测试)
npm run check:ci

# 启动桌面端开发调试环境 (自动启动 Vite 开发服务器并在原生窗口打开)
npm run tauri dev

# 打包发布 Windows 桌面端 Release 二进制
npm run tauri build

# 运行 Android 开发联调 (需要已安装 Android SDK / NDK)
npm run tauri android dev
```
