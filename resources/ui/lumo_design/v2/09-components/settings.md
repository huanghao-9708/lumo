# Settings

> 设置页，从 TopBar "更多" → 设置 进入。

## Sections

### 1. 数据源管理

- 列出所有数据源（本地 / WebDAV），每行显示名称 + 路径 + 删除按钮
- 添加数据源表单：选择类型（本地 / WebDAV）→ 填写名称、路径/URL、认证信息
- 调用 `playerStore.addSource()` / `playerStore.removeSource()`

### 2. 缓存管理

- 显示当前缓存大小（`libraryGetCacheSize()`）
- "清理缓存"按钮（`libraryClearCache()`）

### 3. 主题

- 亮色 / 暗色模式切换（委托 `uiStore.toggleDarkMode()`）
- 与 TopBar 夜间模式按钮同步

### 4. 关于

- 应用名称、版本号、简介

## Navigation

- 从 TopBar "更多"下拉菜单进入（`activeLibraryTab = '设置'`）
- 使用全局后退按钮返回前页

## States

| 状态 | 显示 |
|---|---|
| 加载缓存大小 | "—" |
| 清理中 | "清理中…" |
| 无数据源 | "暂无数据源" |
