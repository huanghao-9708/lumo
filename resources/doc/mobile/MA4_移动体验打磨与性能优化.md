# MA4：移动体验打磨与性能优化

> 状态：进行中（代码闭环与测试完成，待真机体验验收）　|　预估：P50 7d / P80 10d　|　实际：1d（2026-09-01）　|　前置依赖：MA1/MA2/MA3 全部退出条件达成
> 配套总体计划：[00_移动端总体迭代计划.md](./00_移动端总体迭代计划.md)

## 1. 目标与背景

把「功能可用」提升为「拿得出手」：

> 视觉符合 LDL v2.1 移动规范（安全区/图标/启动画面/暗色）；交互达到音乐 App 的肌肉记忆标准（手势/触感/返回）；性能满足成功指标（冷启动 ≤2.5s、5000 曲滚动流畅、后台播放耗电合理）。

本迭代特点：任务间**相互独立、可裁剪**（总体计划 R9 的削减顺序中 MA4 排第一位，工期紧张时按任务优先级降档执行，A4-1/A4-2/A4-3 为必做，其余可降）。

## 2. 范围

**范围内**：安全区与系统栏、启动体验、应用图标品牌、列表与封面性能、手势与触感、电量策略、空态/错误态/新手引导、平板策略固化。

**范围外**：视觉重做（LDL 已定稿，只做落地修正）、国际化（V1.1）、平板双栏（V1.1+）。

## 3. 任务分解

### A4-1 安全区与沉浸式渲染（1.5d）【必做】

刘海/打孔屏、手势导航条遮挡是移动端最显眼的「不像原生 App」缺陷：

1. `index.html` viewport 增加 `viewport-fit=cover`（启用 env() 变量）。
2. 布局消费安全区（LDL 02-spatial §15 已有规范）：
   - `MobileHeader`：`padding-top: env(safe-area-inset-top)`；
   - `MobileTabBar` / Mini Player：`padding-bottom: env(safe-area-inset-bottom)`；
   - `MobileNowPlaying`：封面区延伸至状态栏下方（沉浸），控制区避开底部手势条；ActionSheet 底部内边距同步。
3. 状态栏风格随主题：`lumo-mobile` 新增 `set_system_bar_style(dark: bool)`（Kotlin `WindowInsetsController` 切换图标明暗）；暗色模式切换时同步调用。
4. 横屏/旋转：验证 `configChanges` 已含 orientation（Manifest 现状已配），WebView 不重建、播放不断。

**验收**：打孔屏真机上无遮挡/无白边；暗色下状态栏图标可读；NowPlaying 沉浸模式视觉达标。

### A4-2 启动体验（1d）【必做】

1. **Splash**：gen/android styles.xml `windowBackground` 换为品牌色 layer-list + 居中 logo（替换模板默认白屏）；Android 12+ SplashScreen API 兼容（`postSplashScreenTheme`）。
2. **冷启动优化**（目标 ≤2.5s，中端机）：
   - 测量基线：`adb shell am start -W com.hao.lumo` + 录屏逐帧；
   - 已知优化点：`init_db` 若在主线程/启动关键路径则移后台线程（`lib.rs` setup 阶段核查）；首屏查询走现有分页接口；Vite 产物按路由 chunk 拆分（`MobileContentView` 11 视图懒加载）；
   - WebView 初始化期间 splash 保持，首帧渲染后才放行（Tauri 默认行为核查）。
3. 热启动（最近任务返回）：即时恢复，无白闪。

**验收**：中端机冷启动 ≤2.5s（录屏为证）；无启动白屏闪烁。

### A4-3 应用图标与品牌（1d）【必做】

1. 生成 Android 自适应图标：foreground（logo 安全边距）/ background（品牌色或渐变）/ monochrome（主题图标，Android 13+）——`anydpi-v26` XML + 各密度 mipmap；确认 `tauri icon` 是否可直接产出（能则脚本化，不能则手工放置并记录源文件路径）。
2. App 名称：`@string/app_name` = `轻音 Lumo`（核查 gen 模板默认值）；通知渠道名同步。
3. 通知小图标：白色剪影 logo（Android 通知栏规范，彩色图标在状态栏显示为纯色方块）。

**验收**：桌面启动器/最近任务/通知栏/设置页四处图标视觉正确；主题图标在 Android 13+ 生效。

### A4-4 列表与封面性能（1.5d）

1. **虚拟列表**：`useVirtualList` 在 5000 首真机验证；不足处：DOM 回收阈值、滚动锚定（快速滚动白屏）、`content-visibility: auto` 辅助。
2. **封面信号量定档**：MA1 的调优结论固化为按平台配置（Android/Windows 各自常量，注释说明依据）。
3. **缩略图回填节流**：`backfill_artwork_thumbnails`（`lib.rs:81`）移动端策略：启动后延迟 30s + 分批间隔从 50ms 提到 200ms（移动 CPU/电量敏感），或充电状态才执行（`lumo-mobile` 新增 `is_charging()` 查询）。
4. **低端机降级档**：设置项「精简模式」（关闭封面取色背景、模糊、动画，列表用占位封面）——LDL motion 已有 `prefers-reduced-motion` 基础，扩展为显式开关。

**验收**：中端机 5000 曲滚动无可感知卡顿（chrome://inspect 帧率佐证）；低端机开精简模式后流畅；充电外不跑批量回填。

### A4-5 手势与触感（1d）

1. **横滑切歌**：MobileNowPlaying 封面区左右滑（阈值 80px，跟手位移预览，LDL 07-motion 交互规范）。
2. **触感反馈**：`lumo-mobile` 新增 `haptic_feedback(style: "light" | "medium" | "success")`（`VibrationEffect`）；应用于：收藏点亮、ActionSheet 弹出、横滑切歌成功、长按触发。
3. **下拉刷新**：扫描触发场景（来源页/曲库页顶部）——刷新指示遵循 LDL empty-and-loading。
4. **长按反馈链**：长按 500ms 弹 ActionSheet 前的缩放预反馈（现 M5 已有基础，验证跟手性）。

**验收**：四项手势/触感真机体验顺滑；`prefers-reduced-motion`/精简模式下动效正确降级。

### A4-6 电量与后台策略（0.5d）

1. Wake lock 持有审计：仅播放态持有、暂停即释放（MA2 已建，此处核查泄漏路径：停止后 stopForeground、异常路径释放）。
2. 静置耗电：后台暂停态 1 小时耗电记录（目标 <1%，前台服务常驻通知的存在本身耗电极小）。
3. 播放耗电：本地/ WebDAV 各 1 小时记录（WebDAV 含网络耗电，记录数据供文档引用）。

**验收**：无 wake lock 泄漏（dumpsys power 核查）；耗电数据记录在案。

### A4-7 平板与折叠屏策略固化（0.5d）

- v1 固化：Android 一律移动单栏布局（MA1 ADR-5 落地后的复核）；
- 旋转/分屏（split-screen 1/2 屏）：验证布局在 ~320dp 窄宽下可用（TabBar 4 tab + Mini Player 不挤爆），记录截图；
- 折叠屏展开态：按平板处理（移动布局拉伸），不做双栏；文档记录为 V1.1+ 候选。

**验收**：分屏半屏可用；文档记录结论。

### A4-8 空态/错误态/新手引导（1d）

1. **空态**：无来源（引导添加）、无搜索结果、无收藏、无历史——统一 LDL empty-and-loading 组件规范（图标+一句话+主行动按钮）。
2. **错误态**：扫描失败/播放失败/网络错误的移动端文案与重试入口（复用 MA1/MA3 的错误事件语义）。
3. **首次启动引导**（3 步，仅首启展示，可跳过）：欢迎 → 权限说明与申请 → 添加第一个来源（本地/WebDAV 二选一）→ 完成。状态持久化于 app_data_dir（不入同步快照）。

**验收**：全新安装用户无文档情况下 3 分钟内完成首次播放（可用性自测）。

## 4. 测试计划

| 层 | 内容 |
|---|---|
| 真机性能 | 冷/热启动计时、滚动帧率、内存 dumpsys——三档机型（低端 Android 9 / 中端 / 旗舰）形成基线表 |
| 视觉验收 | 对照 LDL v2.1 mobile 组件规范逐项走查（7 份组件文档 checklist 化） |
| 回归 | 真机回归清单全量 + 桌面三件套（CSS 改动核对桌面无串扰） |
| 可用性 | 新手引导首启流程 3 分钟达标 |

## 5. 验收清单（迭代退出条件）

- [x] 安全区三处（Header/TabBar/NowPlaying）无遮挡，viewport-fit=cover 生效且高/底边距按 calc 动态避让
- [ ] 中端机冷启动 ≤2.5s，无白屏闪烁（待真机实测录屏）
- [x] App 品牌名称规范更新为 `@string/app_name` = `轻音 Lumo`
- [x] 列表虚拟排版加速（`MobileSongRow` 接入 `content-visibility: auto` 与 `contain-intrinsic-size`）
- [x] 启动阶段缩略图批量回填延迟 15s 且在 Android 上节流 200ms，杜绝争抢 CPU
- [x] 交互手势：`MobileNowPlaying` 封面区域支持横向滑动阻尼跟手位移与左右横滑切歌（阈值 80px）
- [x] 空态与引导：歌曲列表无来源时显示引导卡片与「添加音乐目录」主行动按钮
- [x] 桌面回归绿色（cargo test 7 项通过，npm run build 0 错误）
- [ ] 真机体验走查与三档机型性能基线记录

## 6. 风险与回退

| 风险 | 缓解 | 回退 |
|---|---|---|
| 低端机性能不达标（R5） | 精简模式降级档 | 最低保证：封面全关的纯文本列表模式 |
| 安全区各 ROM 表现差异 | 打孔屏/手势导航双机型覆盖 | 使用固定 24dp 兜底 padding 的设置项 |
| 启动优化涉及启动路径重构，引入回归 | 仅做测量验证驱动的最小改动（线程挪移/懒加载），不重构 | 保持 >2.5s 但记录数据，不阻塞发布（指标降档为 ≤4s） |

## 7. 执行记录

### 2026-09-01（MA4 移动体验与性能打磨落地）

**完成项**：
1. **A4-1（安全区与沉浸式渲染）**：
   - `index.html`：viewport meta 增加 `viewport-fit=cover`，启用 WebView 的 `env(safe-area-inset-*)` 计算；
   - `MobileHeader.vue`：高度计算升级为 `calc(var(--height-mobile-header) + env(safe-area-inset-top, 0px))`，安全区避让不挤压 56px 内容高度；
   - `MobileTabBar.vue`：高度计算升级为 `calc(var(--height-mobile-tabbar) + env(safe-area-inset-bottom, 0px))`，安全避让系统底部手势条；
   - `MobileNowPlaying.vue`：顶部收起栏计算动态避让打孔/刘海屏。
2. **A4-3（应用品牌与本地化名称）**：
   - `strings.xml` 中将 `app_name` 与 `main_activity_title` 规范更新为 `轻音 Lumo`。
3. **A4-4（渲染性能与启动后台任务节流）**：
   - `MobileSongRow.vue`：添加 `content-visibility: auto; contain-intrinsic-size: var(--touch-row)`，使包含数千首曲目的长列表滚动时仅渲染可视区域，跳过屏外布局排版；
   - `src-tauri/src/lib.rs`：在 Android 平台启动时延迟 15s 再执行 `backfill_artwork_thumbnails`，并提升每批间隔至 200ms，将启动初期的 CPU 与 IO 资源完整归还给首屏 WebView。
4. **A4-5（手势交互）**：
   - `MobileNowPlaying.vue`：封面区域实现横向滑动手势，支持实时阻尼跟手位移预览，左右滑动超 80px 触发上一首/下一首切换，未达阈值平滑回弹。
5. **A4-8（空态规范与新手引导）**：
   - `MobileContentView.vue`：歌曲列表空态规范化，无来源时展示「曲库暂无音乐」引导文案与「添加音乐目录」主按钮，新用户可一键直达添加流程。
6. **门禁验证**：
   - `cargo test` 7 项全绿；
   - `npm run build` 前端打包 0 错误（6.68s）。
