# MA2：后台播放与系统媒体集成

> 状态：未开始　|　预估：P50 10d / P80 15d　|　前置依赖：MA1 全部退出条件达成
> 配套总体计划：[00_移动端总体迭代计划.md](./00_移动端总体迭代计划.md)
> 对应总体计划 ADR-3（后台播放架构），是移动端**最核心的架构改动**

## 1. 目标与背景

让 Lumo 在 Android 上成为「正经音乐 App」：

> 灭屏/锁屏连续播放 ≥30 分钟（抽样 8 小时不被杀）；通知栏与锁屏五键控制（上一首/播放/暂停/下一首/关闭）+ 进度条可拖；来电自动暂停、挂断恢复；拔出耳机自动暂停；App 被杀后重启可恢复播放现场。

### 为什么必须动架构（问题本质）

当前播放推进由前端驱动：`src/stores/player.ts:1431` 的 `progressTimer = setInterval(...)` 每 500ms 轮询进度，临近曲尾时由 **JS 调用后端命令**预加载下一首（gapless enqueue）。而 Android 在 Activity `onStop`（灭屏/切后台）后会**冻结 WebView 的 JS 定时器与渲染**——当前曲目播完后，没有任何代码去 enqueue 下一首，播放停摆。这不是 bug 修补问题，是职责放错了层。

**方案（ADR-3）**：播放队列在 Rust 侧持有**权威镜像**，Rust 自己监听曲尾、自己推进队列；WebView 前台时是 UX 的唯一入口（用户点歌 → 更新 Rust 队列），后台冻结时 Rust 独立续播；切回前台时前端通过事件与查询接口**对账**。Kotlin 侧只做系统集成（前台服务、MediaSession、通知、音频焦点），**不碰解码**，避免双播放器。

```
┌─────────────── WebView (可冻结) ───────────────┐
│ player.ts: 用户交互入口 · 进度条 UI · 歌词/队列面板 │
└───────┬────────────────────────▲───────────────┘
   play_from / set_queue   playback-track-changed / progress 事件
┌───────▼────────────────────────┴───────────────┐
│ Rust: PlaybackQueue（权威队列镜像 + 模式状态机）    │
│      Sink 空闲监视线程 → 自动推进 + gapless 预载    │
│      PlaybackManager(rodio/cpal) ← MA0 已验证     │
└───────┬────────────────────────▲───────────────┘
   start/update/stop_foreground   media-button 事件
┌───────▼────────────────────────┴───────────────┐
│ Kotlin lumo-mobile: 前台服务 + MediaSession +     │
│ 通知(锁屏控制) + 音频焦点 + 耳机事件 + 开机媒体键    │
└─────────────────────────────────────────────────┘
```

**桌面端不受影响**：Rust 队列对桌面 IPC 同样生效（桌面前端同样改为「设置队列 + 收事件」模式，反而消除现有 gapless 竞态的根源——`enqueuedTrackIndex` 的多处手动重置）；桌面无前台服务调用即可。

## 2. 范围

**范围内**：Rust 队列镜像与推进线程、前端播放器 store 职责重划、前台服务与通知、MediaSession、音频焦点/耳机、播放状态持久化迁移、WebView 冻结对策验证。

**范围外**：锁屏界面视觉定制（仅系统标准媒体通知/锁屏样式）、Android Auto/Wear、均衡器、通知点击深链到指定页面（v1 仅回到 App 主界面）。

## 3. 任务分解

### A2-1 Rust 播放队列镜像（3d）

新文件 `src-tauri/src/services/queue.rs` + `src-tauri/src/commands/queue.rs`：

```rust
pub struct QueueItem {
    pub track_id: i64,
    pub media_file_id: i64,      // 实际播放的版本，历史记录用（审计 P1-03 语义）
    pub play_path: PlaySource,   // Local(PathBuf) | WebDav { url, headers, cached_path: Option<PathBuf> }
    pub title: String, pub artist: String, pub album: String,
    pub artwork_id: Option<i64>, pub duration_ms: Option<u64>,
}

pub struct PlaybackQueue {
    items: Vec<QueueItem>,
    index: usize,
    mode: PlayMode,              // Normal | RepeatAll | RepeatOne | Shuffle
    shuffle_order: Option<Vec<usize>>,
}
```

**新命令**（注册进现有 invoke handler）：

| 命令 | 作用 |
|---|---|
| `playback_set_queue(items, index, mode)` | 用户发起播放/换队列（点歌、拖动队列、切换模式）时全量替换；同时立即播放 index 处曲目并启动前台服务 |
| `playback_queue_state()` | 返回 items/index/mode/position——**前端对账的单一事实来源**（恢复前台、重启后调用） |
| `playback_advance(direction)` | 手动上一首/下一首（通知与前端同一入口） |
| `playback_set_mode(mode)` | 切换播放模式 |

**设计约束**：

1. 队列全量下发（数千首的 ID 列表 JSON，一次 IPC 量级 ~100KB，可接受；超大队列如「全部歌曲 2 万首」实测，超 1MB 则改为「队列描述符 + 分页拉取」，在执行记录中决策）。
2. `record_play`、收藏计数、MediaSession 更新等副作用**统一在 Rust 推进点触发**，消灭「前端忘了调用」类不一致。
3. `PlaybackManager` 增加轨道完成回调能力（轮询 `sink.is_empty()` + 当前轨道游标）。

**验收**：`cargo test` 新增队列状态机单测（四模式 × 首尾边界 × shuffle 可回退不越界——即桌面审计 M2 验收场景的 Rust 化）。

### A2-2 曲尾推进与 gapless 预载监视线程（2d）

Rust 侧 spawn 常驻监视线程（非 WebView 依赖）：

```
loop {
    每 250ms：
      position = sink 当前位置; remaining = duration - position
      if remaining < 3s && 下一首未预载: enqueue_next(resolve(next))   // gapless
      if sink 空了 && 刚完成一曲:                                        // 曲尾
        match mode { Normal 且到队尾 => 停止并停前台服务;
                     RepeatOne => 重播; 其余 => advance() }
        emit("playback-track-changed", { index, track })
        更新 MediaSession 元数据 + 历史入库
      每 1s emit("playback-progress", { position, duration })          // 前端进度条直接消费
}
```

**验收（关键真机场景）**：播放中灭屏，WebView 冻结，曲目正常自动切换连续 30 分钟；切回前台 UI 与实际播放一致（对账见 A2-3）。

### A2-3 前端播放器 store 职责重划（2.5d）

改造 `src/stores/player.ts`（当前约 1910 行，本任务只动播放推进相关段，其余拆分留给桌面质量线）：

1. **删除**：`progressTimer` 的 gapless 预载逻辑（`player.ts:1423-1495` 的 `enqueuedTrackIndex` 及其竞态重置代码）、`saveProgressToStorage` 的 30s 定时器（持久化移到 Rust，见 A2-6）。
2. **保留/新增**：播放/暂停/seek/音量命令不变；`play_from(list, index)` 改为构造 `QueueItem[]` 调 `playback_set_queue`；进度条 UI 改消费 `playback-progress` 事件（`document.visibilityState === 'hidden'` 时跳过渲染）；监听 `playback-track-changed` 更新 currentIndex/歌词/文件信息/MediaSession（桌面）；**恢复前台时**调 `playback_queue_state()` 对账（防事件丢失）。
3. **断电恢复**：启动时调 `playback_queue_state()`——队列与位置已由 Rust 持久化（A2-6），前端重建 UI 即可恢复「上次播放现场」。
4. **兼容**：桌面 Windows 同链路回归（gapless、四模式、媒体键）。

**验收**：桌面与 Android 行为一致；`player.ts` 中不再存在「前端主动 enqueue 下一首」的代码路径。

### A2-4 Kotlin 前台服务与通知（3d）

`lumo-mobile` 插件内新增（Manifest 加 `FOREGROUND_SERVICE`、`FOREGROUND_SERVICE_MEDIA_PLAYBACK`、`POST_NOTIFICATIONS` 权限 + `<service android:name=".MediaPlaybackService" android:foregroundServiceType="mediaPlayback"/>`）：

**插件命令（Rust → Kotlin）**：

| 命令 | 时机 |
|---|---|
| `start_foreground(metadata, state)` | 开始播放 |
| `update_foreground(state, position_ms)` | 播放/暂停/切歌/进度（进度低频推送，seekbar 由 MediaSession PlaybackState 的 position+speed 外推） |
| `stop_foreground()` | 停止播放/退出 |

**Service 职责**（平台 `android.media.session.MediaSession` + `NotificationCompat`，不引入 media3）：

1. `startForeground` 常驻媒体通知：曲目信息 + 封面 Bitmap（从 artwork 缓存路径解码，降采样 512px）+ 五键 + `setProgress`；通知渠道 importance=LOW（不响铃不浮窗）。
2. MediaSession 回调：`onPlay/onPause/onSkipToNext/onSkipToPrevious/onSeekTo/onStop` → 插件向 Rust emit `media-button` 事件 → Rust 执行与前端按钮**完全相同**的代码路径。
3. 通知点击 → 回到 MainActivity（v1 不做深链）。

**Android 14+ 合规**：`foregroundServiceType="mediaPlayback"` 必须显式声明（targetSdk 36）；运行时 `POST_NOTIFICATIONS` 权限被拒时**播放不中断**（仅无通知，后台存活概率下降，文档说明）。

**验收**：通知五键全部生效且与 App 内操作互不踩踏；锁屏展示封面/进度可拖动；快速连点不产生命令乱序（Rust 侧命令幂等）。

### A2-5 音频焦点与耳机事件（1.5d）

Service 内注册：

| 系统事件 | 行为 |
|---|---|
| `AUDIOFOCUS_LOSS`（其它 App 长期抢占） | 暂停 |
| `AUDIOFOCUS_LOSS_TRANSIENT`（来电/导航） | 暂停；`GAIN` 后**不自动恢复**（避免打扰），保留播放态由用户手动恢复——与多数音乐 App 行为对齐 |
| `AUDIOFOCUS_LOSS_TRANSIENT_CAN_DUCK` | 音量降到 20%，`GAIN` 恢复 |
| `ACTION_AUDIO_BECOMING_NOISY`（拔耳机） | 暂停 |
| 媒体键（蓝牙/线控） | MediaSession 已覆盖 |

**验收**：真机来电 → 暂停；挂断不自动续播；拔耳机暂停；蓝牙耳机播放/按键正常。

### A2-6 播放状态持久化迁移（1d）

- Rust 侧：推进/暂停/退出时机将 `{queue, index, mode, position, track}` 写入 `app_data_dir/playback_state.json`（节流：暂停、切歌、每 30s）——替代前端 `saveProgressToStorage`（localStorage 在 Android WebView 冻结时同样不可靠）。
- 启动恢复语义：冷启动后显示「上次播放」态（Mini Player 常驻上次曲目，点播放续播，从持久化 position 继续）。
- 桌面同步切换到此机制（localStorage 写入逻辑保留一版做迁移兼容：首次启动读到旧 localStorage 进度则一次性导入后弃用）。

**验收**：播放中直接杀进程 → 重启 → Mini Player 显示上次曲目 → 点播放从断点续播。

### A2-7 WebView 冻结与进程生命周期对策（1d）

- 灭屏后 WebView 冻结但 **Rust 线程与音频输出继续**（MA0 P-5 已知现象，前台服务保证进程存活）；本任务做系统化验证矩阵：灭屏 / 切其它 App / 最近任务划掉（= 杀进程，验证 A2-6 恢复）/ 系统低内存（`adb shell am send-trim-memory`）。
- Doze 深度休眠（闲置数小时）：前台服务 + 播放中持有 partial wake lock（Service 播放态获取、暂停释放）可穿越 Doze；8 小时灭屏播放抽样一次记录耗电。
- 厂商省电杀后台（R8）：文档提供「省电策略白名单」引导文案（设置页帮助入口），不做代码对抗。

**验收**：验证矩阵全部记录结果；8 小时播放不被杀（或记录到具体厂商行为与耗电数据）。

## 4. 测试计划

| 层 | 内容 |
|---|---|
| Rust 单测 | 队列状态机全模式边界；推进线程决策函数（纯逻辑抽离后测）；持久化 JSON 往返 |
| 前端单测 | 对账逻辑（queue_state → store 同步）、事件丢失场景（恢复前台强制对账） |
| 集成 | `media-button` 事件 → Rust 命令幂等性；通知命令风暴不乱序 |
| 真机 | 灭屏 30min 连播、8h 抽样、来电/拔耳机/蓝牙、杀进程恢复、低内存 |
| 桌面回归 | 四模式、gapless、媒体键、锁屏 MediaSession（Windows 已有实现）不回退 |

## 5. 验收清单（迭代退出条件）

- [ ] 灭屏连续播放 ≥30 分钟，切歌正常（Rust 推进，WebView 冻结无影响）
- [ ] 通知栏：五键 + 进度 + 封面 + 曲目信息，与 App 内状态零偏差
- [ ] 锁屏控制可用，进度可拖动（seek 生效）
- [ ] 来电暂停；拔耳机暂停；蓝牙按键正常
- [ ] 四种播放模式在后台推进下语义正确（Normal 队尾停、RepeatOne 重播等）
- [ ] 杀进程重启恢复播放现场（断点续播）
- [ ] 播放历史记录的 source 为实际播放版本（Rust 统一触发后复验审计 P1-03）
- [ ] 前端不再持有 enqueue 职责，桌面回归全绿
- [ ] Rust 队列状态机单测进入 `cargo test`
- [ ] 真机回归清单执行并记录（含低内存/厂商杀后台场景）

## 6. 风险与回退

| 风险 | 缓解 | 回退 |
|---|---|---|
| 队列全量 IPC 性能不达标 | A2-1 设计约束已列分页预案 | 队列描述符方案（+2d） |
| rodio 后台被 AAudio 断流（部分设备灭屏策略激进） | MA2-7 验证矩阵先行暴露 | cpal 流重建逻辑（推进线程检测 sink 错误自动重建输出流）——预留接口在 `PlaybackManager` |
| MediaSession/通知在各 ROM 表现不一 | 真机矩阵覆盖主流 ROM（MIUI/ColorOS/HyperOS 原生 Pixel 至少各一） | 降级：通知仅三键（去 prev/next）仍保底可用 |
| 前端 store 改造引发桌面回归 | 桌面回归纳入每任务 DoD | 改造分两步合入：先加事件消费（不删旧逻辑，双跑对比一周），再删旧推进路径 |
| 工期超 P80 | MA2 是最大迭代，缓冲已最高 | 通知进度条/锁屏 seek 推迟到 MA4（五键与后台续播不可妥协） |

## 7. 执行记录

> 迭代执行时按日追加。

（待填写）
