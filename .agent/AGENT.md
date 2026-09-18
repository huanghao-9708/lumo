# Lumo 项目 AI 协作开发规范 (Agent Rules)

本文档旨在沉淀项目开发规范，明确 AI 智能体在协助开发本应用时的行为边界与约束原则。所有参与本项目代码编写和维护的 AI 都必须严格遵守此文档。

## 1. 核心技术栈基准

在生成任何代码前，AI 必须基于以下技术栈架构进行思考与实现：

- **桌面端框架**：Tauri v2
- **前端框架**：Vue 3 (基于 Composition API 和 `<script setup>` 语法)
- **状态管理**：Pinia
- **样式引擎**：Tailwind CSS v4 (优先使用 Utility Classes，尽量避免编写自定义 CSS 除非处理如伪类/滚动条等特殊情况)
- **图标库**：lucide-vue-next
- **构建工具**：Vite + TypeScript (`strict` 模式)

## 2. AI 行为边界与限制 (CRITICAL)

### 2.1. 最小破坏原则 (开闭原则)
- **严禁随意重构**：除非用户明确指示，否则绝对不要重构、格式化或修改与当前任务无关的现有代码。
- **保护现有特性**：在添加新功能时，必须保证现有的功能模块不被破坏。修改共享文件（如 `App.vue` 或 `stores`）时，需仔细评估影响范围。

### 2.2. 双端 UI 结构约束
本项目是**桌面端与移动端两套独立 UI**。（早期设想的 `ui-default` / `ui-simple` 插件化多主题架构**从未实现、已废弃**，代码中不存在这两个标识符，勿再据此开发。）

- 桌面：`src/components/layout/` + `src/components/content/`
- 移动：`src/components/mobile/`（独立布局、TabBar、设置页、NowPlaying）
- 共享：`src/components/shared/`、`src/stores/`、`src/api/`、`src/composables/`

规则：
- 改动 `stores` / `api` / 共享组件时，**必须同时核对桌面与移动两端的调用点**。已知移动端缺失入口的能力：智能歌单、AI 电台、批量删除、缓存管理、文件夹视图——补入口前先查 `resources/doc/product/FEATURE_MATRIX.md`。
- 平台差异走 `src/composables/usePlatform.ts` 与 `stores/ui.ts` 的 `isMobile`，**不得**在共享层硬编码某一端布局。
- 新增 IPC 命令须在 `src/api/` 对应领域文件登记参数与返回类型，禁止在组件里裸写 `invoke('xxx')` 字符串。

### 2.3. 环境与安全要求
- **依赖管理**：非必要严禁添加新的 `npm` 依赖包。如果业务逻辑必须引入新依赖，需要先向用户说明原因并获得同意。同时需注意项目具有 `allow-scripts` 相关的安全检查，勿引入高风险包。
- **Tauri 权限限制**：所有使用到 `@tauri-apps/api` 的操作，务必确保在 `src-tauri/capabilities/default.json` 中配置了相应的前端调用权限。

### 2.4. 设计感与视觉要求
本应用高度强调审美与交互体验：
- 不要输出“能用就行”的简陋页面。
- 新增组件必须遵循 LDL 设计语言（`resources/ui/lumo_design/`）：现代感排版，`brand-orange` 作为强调色，阴影与色彩过渡柔和，深浅色双主题同时适配（CSS 变量驱动，勿写死色值）。
- 移动端额外遵守安全区避让与 `--text-*` 字号变量。

## 3. 语言与输出规范
- **中文强制输出**：无论任何情况，AI 在日常沟通、任务规划、逻辑思考、伪影和总结中，均必须使用流畅、专业的**简体中文**。
- **中文注释**：新增逻辑或重构部分的局部代码块需辅以准确的中文注释（专有英文技术词汇如 API、React、UI 等除外）。

---
**提示给 AI Agent：**
每次进入该项目或执行新任务前，请自动回忆本 `agent.md` 中的所有限制，永远把保持现有代码稳定性、遵循上述多主题视觉逻辑和“不擅作主张”放在首位。

## 5. 文档事实源（CRITICAL）

下列文档是**唯一事实源**，禁止在其它文档或 UI 里复制其内容，只做引用：

| 主题 | 事实源 |
|---|---|
| 能力状态与平台承诺等级 | `resources/doc/product/FEATURE_MATRIX.md` |
| 联网行为与隐私边界 | `resources/doc/product/NETWORK_BEHAVIOR.md` |
| 产品定位、术语契约、冻结范围 | `resources/doc/product/PRODUCT_CHARTER.md` |
| 版本、发布通道与门禁 | `resources/doc/release/VERSION_POLICY.md` |
| 关键取舍与例外 | `resources/doc/product/DECISION_LOG.md` |
| 成熟化期间冻结的需求 | `resources/doc/product/POST_STABLE_BACKLOG.md` |

硬性规则：

1. **新增或修改任何外部网络请求**，必须在同一个 PR 内更新 `NETWORK_BEHAVIOR.md`，否则评审打回。
2. **新增数据库迁移**必须附带 fixture 回归测试（I1/QA-003），无测试不得合并；迁移语句需包事务，禁止 `duplicate column` 式不可重跑写法。
3. **禁止**用"同步"描述 WebDAV 数据库能力，统一称"备份恢复"（决策 D-02）；"同步"保留给未来具备实体合并与冲突处理的实现。
4. **禁止**以解码库 features 推断格式支持。对外格式口径只能是 `mp3 flac wav m4a aac`（唯一真源 `src-tauri/src/services/scanner.rs:19`）。
5. **禁止**出现未经 `FEATURE_MATRIX.md` 记录的平台能力声明（尤其"原生支持 Linux/macOS"）。
6. 性能类对外数字（体积、内存、扫描规模）必须来自 `PERFORMANCE_BASELINE.md` 实测区间。
7. `resources/doc/VISION.md` 是**历史文档**，不得作为当前能力依据。
8. 商业成熟化（I0–I6）期间，除 P0 数据损坏、安全漏洞、构建阻断、严重兼容外，**不新增功能**；新需求进 `POST_STABLE_BACKLOG.md` 排队。

## 6. 提交与自检

- 提交前本地必跑：`cargo fmt --all`、`cargo clippy --all-targets -- -D warnings`、`cargo test`、`npx vue-tsc --noEmit`。CI 会重复同样的门禁。
- 版本号只在 `package.json` 手工维护，`Cargo.toml` 与 `tauri.conf.json` 必须与之一致（CI 校验）。
- 纯格式化改动必须独立提交，不得混入行为修改。
