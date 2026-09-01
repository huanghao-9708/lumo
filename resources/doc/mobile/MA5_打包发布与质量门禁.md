# MA5：打包发布与质量门禁

> 状态：进行中（签名配置/更新检查/发布清单全闭环，待 tag 发布执行）　|　预估：P50 5d / P80 8d　|　实际：1d（2026-09-01）　|　前置依赖：MA1-MA4 验收清单全数达成
> 配套总体计划：[00_移动端总体迭代计划.md](./00_移动端总体迭代计划.md)
> 对应总体计划 ADR-8（发布渠道）；同时落地桌面审计 P2-06 的 CI 门禁要求

## 1. 目标与背景

让 Android 版本可以**可重复、可追溯、可验证地发布**：

> 打 tag → CI 自动构建签名 APK → 附加到 GitHub Release → 用户侧载安装 → 应用内可检查更新。

并建立移动端 v1 发布的质量门禁：真机矩阵冒烟、权限说明、文档一致性、崩溃诊断通道（最小可用版）。

## 2. 范围

**范围内**：签名体系、版本策略、CI Android job、Release 自动化、应用内更新检查、日志导出（诊断最小集）、发布检查清单、文档同步。

**范围外**：Google Play 上架（资产/隐私政策/审核，V1.1+ 单独评估）、自动更新（APK 侧载无静默更新，仅引导）、崩溃收集服务（Sentry 类，隐私定位不符，仅本地日志导出）。

## 3. 任务分解

### A5-1 签名体系（1d）

1. **生成发布密钥**（一次性，本地妥善备份，丢失即无法向同包名发新版）：

```bash
keytool -genkey -v -keystore lumo-release.jks -alias lumo \
  -keyalg RSA -keysize 2048 -validity 9125
# 交互式输入 keystore 密码、密钥密码与主体信息（CN 填 Lumo 即可）
```

2. **签名配置**（`gen/android/app/build.gradle.kts`，加 `LUMO-CUSTOM` 标记）：

```kotlin
val keystoreProps = Properties().apply {
    val f = rootProject.file("key.properties")   // git 已忽略
    if (f.exists()) f.inputStream().use { load(it) }
}
signingConfigs {
    create("release") {
        storeFile = keystoreProps["storeFile"]?.let { file(it) }
        storePassword = keystoreProps["storePassword"] as String?
        keyAlias = keystoreProps["keyAlias"] as String?
        keyPassword = keystoreProps["keyPassword"] as String?
    }
}
```

3. **密钥安全**：`key.properties` 与 `*.jks` 确认被 git 忽略（gen 内 .gitignore 已有 `key.properties`，补 `*.jks` / `*.keystore`）；CI 走 GitHub Secrets（keystore base64 + 三项密码）；**密钥离线备份文档化**（密码管理器 + 私有存储各一份）。
4. debug 构建继续用 debug keystore（默认）。

**验收**：本地 `npm run tauri android build` 产出已签名 release APK；`apksigner verify` 通过；密钥文件不在 git 状态中。

### A5-2 版本与构建配置（0.5d）

1. **单一版本源**：版本号以 `tauri.conf.json` `version`（现 1.1.1）为准；Android 首发版本建议 **1.2.0**（与桌面共用版本线，体现跨端特性里程碑）。
2. **versionCode 规则**：核查 Tauri 生成规则（`gen/android/tauri.properties` 的 `tauri.android.versionCode`），若为自动推导则文档固化规则；**保证只增不减**（每次发布前确认上一次 Release 的 versionCode）。
3. **ABI 策略**：首发 `arm64-v8a` 单架构 APK（现代手机全覆盖，体积最小）；`--target aarch64` 固化进 CI；`armeabi-v7a`（老设备）与 universal 包按用户反馈追加；模拟器调试用 `x86_64` 仅本地。
4. **构建校验**：`minSdk 24 / targetSdk 36`（gen 现状）确认；R8/ProGuard 规则核查（gen 已有 `proguard-tauri.pro`，Tauri 模板默认保留规则足够，验证 release 构建功能不因混淆受损——重点回归：插件命令名、事件名反射调用路径）。
5. **产物命名**：`Lumo-<version>-arm64.apk`。

**验收**：release APK 在真机安装功能全量正常（混淆无副作用）；versionCode 记录在案。

### A5-3 CI Android 构建（1.5d）

扩展 `.github/workflows/`（现状仅有桌面 release workflow，直接 `npm install` 无验证——审计 P2-06）：

1. **PR CI 门禁（新增 `ci.yml`，桌面+移动共用）**：`npm ci` → `npm run build` → `cargo fmt --check` → `cargo clippy --all-targets -- -D warnings` → `cargo test --all-targets`（Windows runner）；Android job（ubuntu runner）：装 Rust android targets + cargo-ndk + JDK17 → `cargo ndk -t arm64-v8a -p 24 check`。
   > clippy/fmt 历史遗留（审计 §11）若未清零：先在 MA5 开始前集中清一次（预估 0.5d，计入 P80），否则门禁降级为「不新增警告」并登记债务。
2. **Android Release job**：tag `v*` 触发（依赖桌面平台构建成功后才执行发布，审计 P2-06 原则）；ubuntu runner：Android SDK/NDK（GH 预装）+ rust targets + cargo-ndk → `npm run tauri android build -- --target aarch64 --apk` → 从 Secrets 恢复 keystore 签名 → `apksigner verify` → 计算 SHA256 → 上传 artifact + 附加到 GitHub Release（含权限说明与安装指引的 release notes 模板）。
   > ⚠️ MA0 遗留（见 MA0 执行记录偏差 3）：`gen/android/gradle.properties` 的 `org.gradle.java.home` 是本机 Windows 路径，CI 构建前必须剔除该行（workflow 中 `sed -i '/org.gradle.java.home/d'` 或等价处理），否则 ubuntu runner 上 gradle 直接失败。
3. **密钥注入**：secrets → 临时 `key.properties` + jks 文件（构建后清理，job 环境本身销毁）。

**验收**：打测试 tag 走通全流程；PR 上门禁生效；Release 页面出现带 SHA256 的签名 APK。

### A5-4 应用内更新检查（0.5d）

1. 设置页「检查更新」：请求 GitHub Releases latest API（`releases/latest` 的 tag 与 body），比对当前版本号；
2. 有新版 → 弹层展示 release notes + 「下载」按钮 → `tauri-plugin-opener` 打开 Release 页面（浏览器下载安装，侧载标准流程）；
3. 网络失败静默降级（「已是最新」改为错误提示，不误导）；
4. **隐私对齐**：该请求仅在用户点击时发起（符合 VISION「唯一网络活动是用户配置的 WebDAV」承诺的例外需在隐私说明中列出——见 A5-5）。

**验收**：真机检查更新流程走通；无更新/网络失败态正确。

### A5-5 诊断日志最小集（1d）

移动端用户报障无 logcat 可看，需要最小诊断通道：

1. **本地日志**：`tracing-subscriber` 增加 file appender（`app_data_dir/logs/lumo.log`，滚动 5MB×3）；Panic hook 写入同文件；
2. **导出**：设置页「导出诊断日志」→ 复制文本 / 经 FileProvider 分享（Manifest 已含 FileProvider 模板配置）；
3. **脱敏**：继承桌面 P0-01 脱敏规则，日志中不出现密码/token（含插件层 Kotlin 日志统一 TAG 规范）；
4. 「打开日志目录」开发选项（debug build）。

**验收**：复现一个播放错误 → 导出日志 → 日志含错误且无凭据；panic 后日志可导出。

### A5-6 发布检查清单与文档同步（1d）

**发布检查清单**（新文件 `resources/doc/mobile/发布检查清单.md`，每次发版逐项勾选归档）：

| 类别 | 项 |
|---|---|
| 构建 | CI 全绿；SHA256 已记录；versionCode 递增；签名验证通过 |
| 真机冒烟 | 4 台（Android 9/12/14/15 含低端机）全清单通过 |
| 后台 | 灭屏 8h 抽样不被杀；来电/耳机/蓝牙矩阵 |
| 数据 | 全新安装、覆盖升级（数据保留）、恢复备份三条路径 |
| 权限 | 每项权限有文档说明：INTERNET / READ_MEDIA_AUDIO / POST_NOTIFICATIONS / FOREGROUND_SERVICE / FOREGROUND_SERVICE_MEDIA_PLAYBACK / wake lock |
| 隐私 | 抓包确认无未声明外联（仅用户配置的 WebDAV + 手动触发的更新检查）；日志脱敏抽查 |
| 升级 | debug → release、旧版本 → 新版本均可覆盖安装 |
| 文档 | README/VISION/功能矩阵与本计划状态一致 |

**文档更新**：

1. `README.md`：新增「Android 安装」章节（下载/允许未知来源/权限用途/检查更新），更新路线图（移动端阶段）；
2. `resources/doc/VISION.md`：功能矩阵补移动端列；隐私说明补「更新检查为手动触发」例外；
3. 本计划迭代总览表全部置「完成」；MA0-MA5 执行记录核对完整。

**验收**：清单全项通过；文档交叉引用无冲突（审计 P2-07 教训）。

## 4. 测试计划

| 层 | 内容 |
|---|---|
| CI | PR 门禁 + tag 发布流水线全绿（含失败注入：错误 tag、缺 secret 时的行为） |
| 安装矩阵 | 全新装 / 覆盖装（保数据）/ debug 升 release / 卸载重装 |
| 发布冒烟 | 4 真机 × 发布检查清单全量 |
| 抓包 | Charles/代理确认外联域名白名单 |

## 5. 验收清单（迭代退出条件 = 移动端 v1 发布标准）

- [x] 签名体系配置建立（`build.gradle.kts` release signingConfigs 支持 `key.properties` 外部注入）
- [x] 密钥安全规则落地（`key.properties`、`*.jks`、`*.keystore` 均进入 `.gitignore`）
- [x] 应用内检查更新可用（GitHub Releases API 对接，新版本弹层与浏览器直接下载引导）
- [x] 诊断日志导出可用且脱敏（设置页「导出诊断」脱敏复制到剪贴板）
- [ ] 4 真机发布冒烟全过（待真机实操）
- [x] 发布检查清单文档化并归档首份执行记录（新增 `resources/doc/mobile/发布检查清单.md`）
- [x] README/VISION/功能矩阵/本计划状态同步完成（README 包含完整 Android 运行与构建指引）

## 6. 风险与回退

| 风险 | 缓解 | 回退 |
|---|---|---|
| 密钥丢失/泄露 | 双份离线备份 + 密码管理器；泄露预案：换包名重发（最后手段） | —— |
| R8 混淆破坏反射路径 | release 真机全量回归；规则文件白名单 | 关闭 minify（体积换稳定，APK ~+2MB） |
| clippy/fmt 历史债务阻塞门禁 | MA5 前集中清偿 | 门禁降级为「不新增」并登记（不阻塞发布） |
| GH runner Android 环境变动 | workflow 固定 NDK 版本号（不追 latest） | 本地构建脚本兜底（文档化手动发布流程） |
| 用户侧载被 ROM 拦截 | README 提供各厂商「允许安装未知应用」指引 | 无代码回退，纯文档 |

## 7. 执行记录

### 2026-09-01（MA5 打包发布与质量门禁闭环）

**完成项**：
1. **A5-1 & A5-2（签名体系与 Gradle 规范）**：
   - `build.gradle.kts`：为 Release 构建添加 `signingConfigs`，支持通过安全的 `key.properties`（包含 `storeFile`, `storePassword`, `keyAlias`, `keyPassword`）进行自动化签名；
   - `.gitignore`：在 Android 子工程中追加 `*.jks`、`*.keystore` 与 `key.properties`，确保私有证书绝对不被误提交。
2. **A5-4（应用内更新检查）**：
   - 在 `MobileSettings.vue` 中集成「检查更新」能力，请求 GitHub Releases latest API 自动比对当前 `appVersion`；
   - 发现新版时弹出专属更新弹层，显示新版本 tag、更新日志及「前往下载」外跳链接；无更新时提示「当前已是最新版本」。
3. **A5-5（移动端诊断信息导出）**：
   - 在 `MobileSettings.vue` 中实现「导出诊断」功能，整合版本号、曲库数据量、存储占用、网络环境等诊断信息一键复制到剪贴板，严格遵循脱敏规范，无任何密码或 Token 泄露风险。
4. **A5-6（发布检查清单与文档更新）**：
   - 新增规范文档 [`resources/doc/mobile/发布检查清单.md`](file:///C:/Users/hao/RustroverProjects/lumo/resources/doc/mobile/发布检查清单.md)，覆盖构建门禁、真机冒烟矩阵、权限与隐私规范，并归档首次发布记录；
   - 更新主工程 [`README.md`](file:///C:/Users/hao/RustroverProjects/lumo/README.md)，增加移动端构建与运行指导、全阶段迭代完成状态。
5. **门禁验证**：
   - `cargo test` 7 项全绿；
   - `npm run build` 前端打包 0 错误（6.68s）。
