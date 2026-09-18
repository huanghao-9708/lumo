#!/usr/bin/env node
/**
 * 版本号单一来源校验（resources/doc/release/VERSION_POLICY.md §8）。
 *
 * 三处版本必须完全一致：
 *   package.json            —— 唯一登记处
 *   src-tauri/Cargo.toml    —— Rust crate 版本（产物 FileVersion / 更新器比对依据）
 *   src-tauri/tauri.conf.json —— Tauri 打包与 updater 读取的版本
 *
 * Android 不单独校验：gen/android/app/build.gradle.kts 的 versionName/versionCode 来自
 * `tauri.android.versionName` 属性，由 tauri build 从 tauri.conf.json 注入，无第二处硬编码。
 *
 * 不一致的历史代价：1.8.0 发布时 Cargo.toml 落后一位，Windows 安装器显示 1.7.0，
 * 用户以为没升级成功又装了一遍。这里把它变成 CI 门禁，而不是发布后的答疑。
 *
 * 用法：node scripts/check-versions.mjs （npm run check:versions）
 */
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');

const SEMVER = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-((?:0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*)(?:\.(?:0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*))*))?(?:\+([0-9a-zA-Z-]+(?:\.[0-9a-zA-Z-]+)*))?$/;

function readPackageJson() {
  const raw = readFileSync(join(root, 'package.json'), 'utf8');
  return JSON.parse(raw).version;
}

/**
 * Cargo.toml 无 JSON 结构，取 [package] 段里的 version = "x.y.z"。
 * 必须显式定位 [package] 再截到下一个 section：按"第一个 section"解析的话，
 * 一旦 [package] 不再置顶，就会读到依赖行的 version = "0.32" 并静默通过。
 */
function readCargoVersion() {
  const raw = readFileSync(join(root, 'src-tauri', 'Cargo.toml'), 'utf8');
  const start = raw.match(/^\[package\][ \t]*$/m);
  if (!start) return undefined;
  const body = raw.slice(start.index + start[0].length);
  const nextSection = body.search(/^\s*\[[^\]]+\]\s*$/m);
  const packageSection = nextSection === -1 ? body : body.slice(0, nextSection);
  const match = packageSection.match(/^\s*version\s*=\s*"([^"]+)"/m);
  return match ? match[1] : undefined;
}

function readTauriConfigVersion() {
  const raw = readFileSync(join(root, 'src-tauri', 'tauri.conf.json'), 'utf8');
  return JSON.parse(raw).version;
}

const sources = [
  { label: 'package.json', version: readPackageJson() },
  { label: 'src-tauri/Cargo.toml', version: readCargoVersion() },
  { label: 'src-tauri/tauri.conf.json', version: readTauriConfigVersion() },
];

const problems = [];

for (const { label, version } of sources) {
  if (!version) {
    problems.push(`${label}: 读取不到 version 字段`);
  } else if (!SEMVER.test(version)) {
    problems.push(`${label}: "${version}" 不是合法 SemVer（x.y.z[-prerelease][+build]）`);
  }
}

const declared = sources.map((s) => s.version).filter(Boolean);
if (new Set(declared).size > 1) {
  problems.push(
    '三处版本不一致：\n' +
      sources.map((s) => `  ${s.label.padEnd(28)} ${s.version ?? '<missing>'}`).join('\n'),
  );
}

for (const { label, version } of sources) {
  console.log(`${version === declared[0] ? '  ok  ' : ' FAIL '} ${label.padEnd(28)} ${version ?? '<missing>'}`);
}

if (problems.length) {
  console.error('\n版本一致性检查失败：');
  for (const p of problems) console.error(` - ${p}`);
  console.error('\n修复方式：只在 package.json 改版本号，然后同步到另外两处（VERSION_POLICY.md §8）。');
  process.exit(1);
}

console.log(`\n版本单一来源校验通过：Lumo v${declared[0]}`);
