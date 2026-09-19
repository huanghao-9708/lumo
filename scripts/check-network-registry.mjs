#!/usr/bin/env node
/**
 * 联网行为登记校验（resources/doc/product/NETWORK_BEHAVIOR.md 的机器门禁，CR-008 起为**主机级**）。
 *
 * 产品契约里「默认拒绝联网、每一处外联都可查证」是最容易被后续提交悄悄破坏的一条：
 * 加一行 reqwest::get() 或 fetch() 不会让任何现有检查变红。本脚本把「新增外联必须同 PR
 * 更新 NETWORK_BEHAVIOR.md」变成可执行规则——CI 直接失败，而不是靠 reviewer 记性。
 *
 * 判定粒度（旧的「文件里出现过 http 就算登记过」已在 CR-008 中废弃）：
 *  1. Rust 代码行里出现的字面量主机，必须逐主机命中 §1；在已登记文件中新增域名同样会红。
 *  2. 真正发起请求的调用点（Rust 的 HTTP client 动词 / 前端的 fetch、window.open、WebSocket 等）
 *     必须带 `联网行为: §2X` 声明；主机由变量、配置或响应决定的动态目标只能靠这条被钉住。
 *  3. 前端与 Rust 共用同一份 §1/§2 事实源；纯本机协议（lumo://）用 `联网行为: local` 显式声明。
 *  4. 回环与保留域（127.x、localhost、::1、*.local、example.*）视为测试夹具，不算外联；
 *     Rust 的 `#[cfg(test)]` 整段、前端 `*.spec.ts` 同样跳过。
 *
 * 判定逻辑在 network-registry-rules.mjs（纯函数，由 vitest 正反例覆盖），本文件只做文件遍历与输出。
 *
 * 用法：node scripts/check-network-registry.mjs （npm run check:network）
 */
import { readFileSync, readdirSync, statSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join, relative, sep } from 'node:path';

import { parseRegistry, scanFile, danglingHosts, formatViolation } from './network-registry-rules.mjs';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const RUST_SRC = join(root, 'src-tauri', 'src');
const FRONTEND_SRC = join(root, 'src');
const DOC = join(root, 'resources', 'doc', 'product', 'NETWORK_BEHAVIOR.md');

function* walk(dir, matches) {
  for (const entry of readdirSync(dir)) {
    const path = join(dir, entry);
    if (statSync(path).isDirectory()) {
      if (entry === 'node_modules' || entry === 'dist' || entry === 'target') continue;
      yield* walk(path, matches);
    } else if (matches(path)) {
      yield path;
    }
  }
}

const isTestFile = (path) => /\.(spec|test)\.[mc]?[jt]sx?$/.test(path);

const targets = [
  ...walk(RUST_SRC, (p) => p.endsWith('.rs')).map((p) => ({ path: relative(root, p).split(sep).join('/'), kind: 'rust' })),
  ...walk(FRONTEND_SRC, (p) => /\.(ts|js|vue)$/.test(p) && !isTestFile(p)).map((p) => ({
    path: relative(root, p).split(sep).join('/'),
    kind: 'frontend',
  })),
];

if (targets.length === 0) {
  console.error('未扫描到任何源文件——请确认脚本路径是否随目录结构调整。');
  process.exit(1);
}

const registry = parseRegistry(readFileSync(DOC, 'utf8'));
if (registry.rows.size === 0) {
  console.error(`未能从 ${relative(root, DOC).split(sep).join('/')} 解析出 §1 外联目标总览表。`);
  console.error('门禁依赖该表的主机白名单；表结构（`| A | 目标 | \\`域名\\` |`）变化时请同步本脚本。');
  process.exit(1);
}

const violations = [];
const declared = [];
for (const target of targets) {
  const content = readFileSync(join(root, target.path), 'utf8');
  const result = scanFile({ ...target, content, registry });
  violations.push(...result.violations);
  declared.push(...result.declared);
}

const hosts = [...registry.allHosts.keys()].sort();
console.log(`事实源：${relative(root, DOC).split(sep).join('/')}（§1 登记主机 ${hosts.length} 个：${hosts.join(', ' || '无')}）`);
console.log(`扫描范围：${targets.length} 个源文件（Rust ${targets.filter((t) => t.kind === 'rust').length} / 前端 ${targets.filter((t) => t.kind === 'frontend').length}）`);
console.log(`已声明行为 ID 的调用点 ${declared.length} 处：`);
for (const d of declared) {
  console.log(`  ${d.file}:${d.line} → ${d.local ? 'local' : d.ids.map((i) => `§2${i}`).join(' + ')}`);
}

const dangling = danglingHosts(registry, targets.map((t) => ({ content: readFileSync(join(root, t.path), 'utf8') })));
if (dangling.length) {
  console.log('\n提示：以下 §1 主机未在任何源码字面量中出现（响应内 CDN 或用户自填地址属正常，其余请确认能力是否已下线）：');
  for (const { host, ids } of dangling) console.log(`  §2${ids.join('/§2')} ${host}`);
}

if (violations.length) {
  console.error('\n联网行为登记校验失败：');
  for (const v of violations) console.error(` - ${formatViolation(v)}`);
  console.error(
    '\n处理方式：\n' +
      '  · 新主机 → 在 NETWORK_BEHAVIOR.md §1 增行、§2 增小节（触发条件、发送字段、默认开关、落地位置）。\n' +
      '  · 调用点缺声明 → 在该行上方或行尾加 `// 联网行为: §2X`；X 必须是 §1/§2 都存在的 ID。\n' +
      '  · 不出网的动态 URL（如 lumo:// 自定义协议）→ 声明 `// 联网行为: local`。\n' +
      '若这是误报，请修正 network-registry-rules.mjs 的判定并补一条 vitest 反例，而不是关掉这个检查。'
  );
  process.exit(1);
}

console.log('\n联网行为登记校验通过：主机级白名单 + 调用点行为 ID 声明，前后端共用同一事实源。');
