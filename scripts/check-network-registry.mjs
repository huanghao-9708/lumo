#!/usr/bin/env node
/**
 * 联网行为登记校验（resources/doc/product/NETWORK_BEHAVIOR.md 的机器门禁）。
 *
 * 产品契约里"默认拒绝联网、每一处外联都可查证"是最容易被后续提交悄悄破坏的一条：
 * 加一行 reqwest::get() 不会让任何现有检查变红。本脚本把"新增外联必须同 PR 更新
 * NETWORK_BEHAVIOR.md"变成可执行规则——CI 直接失败，而不是靠 reviewer 记性。
 *
 * 判定方式：Rust 源码里出现以字符串字面量形式书写的 http(s):// 目标，
 * 即视为一次外联点；该文件必须被 NETWORK_BEHAVIOR.md 按路径点名。
 *
 * 用法：node scripts/check-network-registry.mjs （npm run check:network）
 */
import { readFileSync, readdirSync, statSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join, relative, sep } from 'node:path';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const SRC = join(root, 'src-tauri', 'src');
const DOC = join(root, 'resources', 'doc', 'product', 'NETWORK_BEHAVIOR.md');

/** 字符串字面量里的 URL：`"https://…` / `b"https://…` / `format!("https://…` */
const URL_IN_STRING = /["']https?:\/\//;

function* walk(dir) {
  for (const entry of readdirSync(dir)) {
    const path = join(dir, entry);
    if (statSync(path).isDirectory()) yield* walk(path);
    else if (path.endsWith('.rs')) yield path;
  }
}

/** 只统计真正参与编译的行：跳过 `//` 注释与 doc 注释里的示例链接 */
function isCodeLine(line) {
  const t = line.trim();
  return !(t.startsWith('//') || t.startsWith('/*') || t.startsWith('*'));
}

function outboundFiles() {
  const hits = new Map();
  for (const file of walk(SRC)) {
    const lines = readFileSync(file, 'utf8').split('\n');
    const found = [];
    lines.forEach((line, i) => {
      if (isCodeLine(line) && URL_IN_STRING.test(line)) found.push(i + 1);
    });
    if (found.length) hits.set(relative(SRC, file).split(sep).join('/'), found);
  }
  return hits;
}

const doc = readFileSync(DOC, 'utf8');
const hits = outboundFiles();

const undeclared = [...hits.entries()].filter(([path]) => !doc.includes(path));

if (hits.size === 0) {
  console.error('未扫描到任何外联点——这本身可疑，请确认脚本路径是否随目录结构调整。');
  process.exit(1);
}

console.log(`扫描到 ${hits.size} 个含硬编码 http(s) 目标的源文件：`);
for (const [path, lines] of hits) {
  const mark = undeclared.some(([p]) => p === path) ? ' UNDECLARED' : '   declared   ';
  console.log(`${mark} ${path}:${lines.join(',')}`);
}

if (undeclared.length) {
  console.error('\n发现未在联网清单中登记的外联点：');
  for (const [path, lines] of undeclared) {
    console.error(` - ${path} (行 ${lines.join(', ')})`);
  }
  console.error(
    '\n请在 resources/doc/product/NETWORK_BEHAVIOR.md 补登记（触发条件、发送字段、默认开关、落地位置），' +
      '并把该行号一起写进 PR 描述。若这是误报（例如仅用于测试的假 URL），' +
      '把该行的 URL 挪进注释或测试专用常量，而不是关掉这个检查。'
  );
  process.exit(1);
}

console.log('\n联网行为登记校验通过：每一处硬编码外联目标都在公开清单中。');
