# Lumo 版本与发布通道政策（VERSION_POLICY）

> 文档状态：v1.0 待确认（I0 / PC-005）
> 生效范围：Lumo 全部平台产物
> 上位文档：`../product/PRODUCT_CHARTER.md`（平台承诺分级）、`../product/FEATURE_MATRIX.md`（能力状态）

## 1. 现状问题（本政策的出发点）

| 问题 | 证据 |
|---|---|
| 版本号多处手工维护 | `package.json:4`、`src-tauri/Cargo.toml:3`、`src-tauri/tauri.conf.json:4` 三处独立字符串；前端 UI 已经通过 vite 注入收敛到 `package.json`（`src/config/appInfo.ts`），但 Rust 侧无一致性校验 |
| 同日连发多个次版本 | 2026-09-12 一天内发布 1.5.0 / 1.5.1 / 1.6.0 / 1.7.0 / 1.8.0 / 1.8.1 共 6 个版本（`git log` 中 `chore(release): bump version`） |
| 版本号不代表稳定程度 | 1.2.0（Android）与 1.8.1（桌面）同号不同能力面；macOS/Linux 从未发版却随 CI 构建产出物 |
| 发布绕开门禁 | `release.yml` 由 tag 直接触发，此前无 PR CI，即"任意提交打 tag 即对外发布" |
| 无支持窗口 | 用户报告 1.5.x 问题时无法判断是否在支持范围内 |

## 2. 发布通道

| 通道 | 受众 | 构建来源 | 更新提示 | 数据兼容要求 | 允许暴露未验证平台 |
|---|---|---|---|---|---|
| **Nightly** | 贡献者、愿冒风险的用户 | `main` 每日定时构建 | 不做任何通知 | 无（可能损坏，需自备备份） | 是（含 macOS/Linux 技术预览） |
| **Beta** | 外部测试者（I6 的 10–20 人） | 打 `v*-beta.*` tag | 应用内可提示，明确标 Beta | 必须可从上一 Stable 升级；不允许不可逆迁移 | 否 |
| **Stable** | 全体用户 | 打 `v*` tag，**且该 commit 必须已通过完整 PR CI** | 应用内更新检查 | 必须满足 §5 数据规则 | 否 |

**通道纪律**：
1. 未经 §4 门禁的构建**不得**放到 Stable 通道；macOS/Linux 在补齐 `FEATURE_MATRIX.md` V-02/V-03 验证前，产物只进 Nightly，且文件名带 `-preview` 后缀。
2. Android 在真机冒烟矩阵（`FEATURE_MATRIX.md` G-01）完成前，Stable 通道只发 **预览版**，Release Notes 首行必须写明"未经真机验证"。
3. 同一时间只有一个 Stable 版本对外可见；不并行维护两条 Stable 线（除非进入 §6 支持窗口的旧线安全维护）。

## 3. SemVer 规则（Lumo 特化）

版本号格式 `MAJOR.MINOR.PATCH`，预发布 `MAJOR.MINOR.PATCH-beta.N`。

| 位 | 递增条件 | Lumo 具体判定 |
|---|---|---|
| **MAJOR** | 破坏性变更 | ① 数据库 schema 迁移**不可回退**到上一版本；② IPC 命令契约破坏性变更；③ 备份快照格式不兼容；④ 支持平台集合收缩（如移除 Windows x64） |
| **MINOR** | 新增用户可见能力 | 新格式入库、新平台正式支持、新视图 |
| **PATCH** | 缺陷修复、性能与安全 | 不改用户可见能力集合；文档-only 变更不占用版本号（直接提交） |

**新增判定（针对本项目历史问题）**：

1. **一个 MINOR 至少承载一项独立的用户可见改进**，不得把三天的多个特性打包进同一 MINOR 后连续跳号。
2. **同一天不允许发布两个及以上 MINOR 版本**。若一天内产生多个特性提交，说明它们应合为一个 MINOR 或推迟到下一周期。
3. **Stable 发版节奏**：MINOR 版本之间至少间隔 **7 个自然日**；PATCH 不受此限制，但每周不超过 2 次。超出该节奏的发版请求需在 `DECISION_LOG.md` 登记例外。
4. **禁止把 `chore`、`docs`、`test`、`refactor` 类提交计入 MINOR**。
5. 版本号一旦发布到 Stable **不得回收或复用**，即使该版本被撤回。

## 4. 发布门禁（Stable 必须全部满足）

```text
[必需] PR CI 全绿（fmt / clippy -D warnings / cargo test / vue-tsc / vite build / 前端测试 / 版本一致性）
[必需] 迁移回归测试通过：至少 3 个历史库 fixture 升级 + integrity_check + 重复启动幂等
[必需] 无未关闭的 P0（数据损坏、凭据泄露、无法启动、无法发布）
[必需] 高危依赖漏洞为零，或已登记豁免且有复审日期
[必需] 联网清单（NETWORK_BEHAVIOR.md）与本次变更一致
[必需] 产物签名可验证（Android apksigner；Windows/macOS 在 I4 补齐前如实标注未签名）
[必需] Release Notes 从本文件 + FEATURE_MATRIX 生成，不手写第二套功能清单
[必需] 完成一次真实安装 + 一次升级 + 一次回滚演练（I4 起强制）
```

任一项不满足时，**只允许发 Beta 或 Nightly**，不得发 Stable。

## 5. 数据迁移与版本兼容性规则

1. **每个 Stable 版本必须声明可升级的最低历史版本**，写入 `CHANGELOG` 与本文件 §8 表。
2. **同一 MAJOR 线内，迁移必须可被旧客户端读取**（不允许删列、不允许改语义），否则视为破坏性变更 → MAJOR。
3. **禁止在迁移中执行不可逆数据改写而不留回滚副本**（针对 `FEATURE_MATRIX.md` G-02/G-05）。
4. 备份快照须携带 schema 版本与 checksum；客户端**拒绝**恢复高于自身识别范围的库（`FEATURE_MATRIX.md` G-04）。
5. 新增迁移必须附带 fixture 测试，无测试的迁移不得合并（I1 QA-003 强制）。

## 6. 支持窗口

| 通道 | 支持内容 | 期限 |
|---|---|---|
| 当前 Stable | 缺陷修复、安全修复、兼容适配 | 直至下一个 Stable 发布后 90 天 |
| 上一 Stable | 仅安全修复与 P0 数据风险 | 直至当前 Stable 后两个 MINOR |
| Beta | 尽力而为，不承诺 | 版本被 Stable 取代即终止 |
| Nightly | 不承诺支持 | 滚动 |
| 技术预览平台（macOS/Linux） | 不承诺 | 完成验证前 |

**支持窗口外的版本**：复现步骤要求先在当前 Stable 验证；不在窗口内的功能请求直接关闭。

## 7. 热修复（hotfix）政策

仅以下四类可申请热修复：

1. 数据损坏或数据丢失（P0）；
2. 凭据泄露 / 远程代码执行 / 证书或签名失效（P0）；
3. 升级后无法启动（P0）；
4. 无法构建或无法回滚上一版本（P0）。

**热修复最小验证集**（不得缩减）：

- `cargo test` + `vue-tsc` + 构建产物可安装；
- 受影响链路的复现步骤在修复包上人工验证通过；
- 若涉及迁移：三个历史 fixture 升级 + `PRAGMA integrity_check`；
- 若涉及凭据/网络：`NETWORK_BEHAVIOR.md` 差异确认；
- 发布后 48 小时内补齐正式回归测试，否则下一个版本被阻断。

**热修复禁止**夹带功能改动、格式化、依赖升级（安全漏洞修复除外）。

## 8. 版本号单一来源与一致性检查

- **声明源**：`package.json` 的 `version` 字段为唯一人工维护点；`src-tauri/Cargo.toml` 与 `src-tauri/tauri.conf.json` 必须与之相等；Android `versionCode` 由 Tauri 从版本号派生（`gen/android/tauri.properties`），只增不减。
- **自动检查**：I1 交付 `scripts/check-version.mjs`（或 shell 等价物），CI 中执行，比对三处字符串完全一致且 Android versionCode 严格大于上一发布值；不一致即红灯。
- **发版流程**：改版本号必须是独立提交（`chore(release): bump version to X.Y.Z`），不得与功能代码混在同一提交，便于回滚与审计。

## 9. 命名与术语纪律

| 场景 | 用词 |
|---|---|
| 对外发布物 | `Lumo <version>` + 平台 + 架构，Android 追加 `Lumo-<ver>-arm64.apk`（沿用现有 `release.yml:134-146` 命名） |
| 未签名平台 | 文件名带 `-preview`，Release Notes 标注 |
| 备份能力 | **备份恢复**，禁止"同步"（`DECISION_LOG.md` D-02） |
| AI 能力 | **AI 电台（Beta，需自备 API Key）** |
| 在线元数据 | 「在线歌词匹配 / 在线封面匹配」，附服务商名（`NETWORK_BEHAVIOR.md`） |
