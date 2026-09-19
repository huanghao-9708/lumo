import { describe, expect, it } from 'vitest';

import {
  danglingHosts,
  isLoopbackHost,
  parseRegistry,
  scanFile,
} from './network-registry-rules.mjs';

/**
 * 联网登记门禁的判定逻辑测试（CR-008）。
 *
 * 这里测的不是「脚本能不能跑」，而是三条验收标准是否真的成立：
 * 已登记文件新增域名必须失败、动态调用点缺行为 ID 必须失败、前后端共用同一事实源。
 * 没有这批反例，门禁和红鲱鱼只差一次「忘记更新清单」的提交。
 */

const DOC = `
## 1. 外联目标总览

| # | 目标 | 域名 | 用途 |
|---|---|---|---|
| A | lrclib | \`lrclib.net\` | 在线歌词 |
| B | 网易云 | \`music.163.com\` + 响应内图片 CDN | 封面 |
| C | iTunes | \`itunes.apple.com\` | 封面兜底 |
| D | AI 服务商 | 用户自填 \`base_url\` | AI 电台 |
| E | 只在 §1 出现 | \`only-in-table.test\` | 缺 §2 小节 |
| G | GitHub | \`api.github.com\` | 更新检查 |

## 2. 逐项明细

### A. 在线歌词
### B. 在线封面
### C. 封面兜底
### D. AI 电台
### G. 更新检查
`;

const registry = parseRegistry(DOC);

const rust = (content) => scanFile({ path: 'src-tauri/src/services/x.rs', kind: 'rust', content, registry });
const front = (content, path = 'src/components/Widget.vue') =>
  scanFile({ path, kind: 'frontend', content, registry });

describe('parseRegistry', () => {
  it('把 §1 每行解析成「行为 ID → 主机集合」，并要求 §2 小节存在', () => {
    // E 行的 `only-in-table.test` 照样进白名单：解析阶段不做豁免，
    // 「保留域不算外联」发生在扫描阶段，这样清单本身永远可读。
    expect([...registry.allHosts.keys()].sort()).toEqual(
      ['api.github.com', 'itunes.apple.com', 'lrclib.net', 'music.163.com', 'only-in-table.test'].sort(),
    );
    expect(registry.rows.get('A').hosts.has('lrclib.net')).toBe(true);
    // E 只有 §1 行没有 §2 小节：登记不完整
    expect(registry.rows.get('E').section).toBe(false);
    expect(registry.rows.get('G').section).toBe(true);
  });

  it('不会把 `base_url` 这类配置字段名当主机', () => {
    expect(registry.rows.get('D').hosts.size).toBe(0);
    expect(registry.allHosts.has('base_url')).toBe(false);
  });
});

describe('主机级白名单（验收标准一）', () => {
  it('已登记文件里新增未登记域名 → 失败', () => {
    // 旧的门禁只看「这个文件有没有被清单点名」：lrclib.rs 已登记，
    // 于是同一文件里新加的第二个主机可以悄悄跟着上线。现在按主机逐个判。
    const result = rust(`
const TRACKER: &str = "https://lrclib.net/api/get";
const NEW: &str = "https://cdn.new-service.io/ping";
`);
    expect(result.violations.map((v) => v.line)).toEqual([3]);
    expect(result.violations[0].detail).toContain('cdn.new-service.io');
  });

  it('回环 / 保留域是测试夹具，不算外联', () => {
    for (const host of [
      '127.0.0.1',
      'localhost',
      'lumo.localhost',
      '::1',
      '[::1]',
      '0.0.0.0',
      'nas.local',
      'example.com',
    ]) {
      expect(isLoopbackHost(host)).toBe(true);
    }
    expect(isLoopbackHost('music.163.com')).toBe(false);
    const result = rust(`
    let base = format!("http://{}", addr);
    let probe = "http://127.0.0.1:8080/dav";
`);
    expect(result.violations).toEqual([]);
  });

  it('Rust 的 #[cfg(test)] 段整体豁免，段外仍要登记', () => {
    const result = rust(`
const PROD: &str = "https://prod-only.internal/x";
#[cfg(test)]
mod tests {
    const FIXTURE: &str = "https://fixture-only.internal/x";
}
`);
    expect(result.violations).toHaveLength(1);
    expect(result.violations[0].line).toBe(2);
  });

  it('调用点写死的主机必须落在所声明那一行里，不能借别的条目蒙过去', () => {
    const ok = rust(`
    // 联网行为: §2A
    let resp = client.get("https://lrclib.net/api/get");
`);
    expect(ok.violations).toEqual([]);

    const mismatch = rust(`
    // 联网行为: §2A
    let resp = client.get("https://itunes.apple.com/search");
`);
    expect(mismatch.violations.map((v) => v.type)).toContain('host-row-mismatch');
  });
});

describe('调用点行为 ID 声明（验收标准二）', () => {
  it('动态 URL 调用点没有声明 → 失败', () => {
    const result = rust(`
    let client = reqwest::Client::builder();
    let resp = client.get(user_supplied_url);
`);
    expect(result.violations.map((v) => v.type)).toEqual([
      'missing-behavior-id',
      'missing-behavior-id',
    ]);
  });

  it('声明后通过；标记可以放在上一行，也可以放行尾', () => {
    expect(
      rust(`
    // 联网行为: §2D —— base_url 由用户自填
    let mut req = client.post(&url).json(&body);
    let bulk = client.put(file_url); // 联网行为: §2B §2C
`)
        .violations,
    ).toEqual([]);
  });

  it('跨行链式调用逃不掉：客户端与动词分行同样是调用点', () => {
    const result = rust(`
    let resp = client
        .get(user_supplied_url)
        .send()
        .await?;
`);
    expect(result.violations).toHaveLength(1);
    expect(result.violations[0].line).toBe(2);
  });

  it('一个标记不能罩住多处请求', () => {
    const result = rust(`
    // 联网行为: §2E
    let a = client.get(url_a);
    let b = client.get(url_b);
`);
    expect(a_violations_of(result)).toEqual([{ line: 4 }]);
  });

  it('标记离调用点太远（隔了代码行）不再向上继承', () => {
    const result = rust(`
    // 联网行为: §2E
    let unrelated = compute();
    let b = client.get(url_b);
`);
    expect(a_violations_of(result)).toEqual([{ line: 4 }]);
  });

  it('引用不存在的 ID、或只有 §1 行没有 §2 小节的 ID → 失败', () => {
    const unknown = rust(`
    // 联网行为: §2Z
    let b = client.get(url);
`);
    expect(unknown.violations.map((v) => v.type)).toContain('unknown-behavior-id');

    const incomplete = rust(`
    // 联网行为: §2E
    let b = client.get(url);
`);
    expect(incomplete.violations.map((v) => v.type)).toContain('unknown-behavior-id');
  });

  it('本地协议（lumo://）显式声明 local 即可，不占用行为 ID', () => {
    const result = front(`
  // 联网行为: local —— lumo:// 自定义协议，不出网
  const resp = await fetch(url);
`);
    expect(result.violations).toEqual([]);
    expect(result.declared[0].local).toBe(true);
  });

  it('客户端构造点也要声明：那是「去哪」被决定的地方', () => {
    const result = rust(`
    let webdav = WebdavClient::new(root_uri.clone(), username, password);
`);
    expect(result.violations.map((v) => v.type)).toEqual(['missing-behavior-id']);
  });
});

describe('前后端共用同一事实源（验收标准三）', () => {
  it('前端 fetch 里的硬编码主机同样按 §1 校验', () => {
    const result = front(`
  // 联网行为: §2G
  const res = await fetch('https://api.github.com/repos/x/y/releases/latest');
`);
    expect(result.violations).toEqual([]);

    const sneaky = front(`
  // 联网行为: §2G
  const res = await fetch('https://telemetry.vendor.io/v1/ping');
`);
    // 未登记的主机只有一种错：清单里没有它。再叠一条「不属于 §2G」是重复告警。
    expect(sneaky.violations.map((v) => v.type)).toEqual(['undeclared-host']);
  });

  it('前端 URL 先存进变量也必须经过主机白名单', () => {
    const sneaky = front(`
<script setup lang="ts">
const endpoint = 'https://telemetry.vendor.io/v1/ping';
// 联网行为: §2G
await fetch(endpoint);
</script>
`);
    expect(sneaky.violations.map((v) => v.type)).toEqual(['undeclared-host']);

    const registered = front(`
<script setup lang="ts">
const endpoint = 'https://api.github.com/repos/x/y/releases/latest';
// 联网行为: §2G
await fetch(endpoint);
</script>
`);
    expect(registered.violations).toEqual([]);
  });

  it('前端只扫真正的调用点：设置页 placeholder 不是外联', () => {
    const result = front(`
<template>
  <input placeholder="https://api.openai.com/v1" />
  <input placeholder="http://nas.local/dav" />
</template>
<script setup lang="ts">
function openRelease(url: string) {
  // 联网行为: §2G
  window.open(url, '_blank');
}
</script>
`);
    expect(result.violations).toEqual([]);
  });

  it('Tauri dialog 的 open() 与 window.open() 不是一回事', () => {
    const dialog = front(`
  const selected = await open({ directory: true });
`);
    expect(dialog.violations).toEqual([]);
    const nav = front(`
  const selected = await window.open(someUrl);
`);
    expect(nav.violations.map((v) => v.type)).toEqual(['missing-behavior-id']);
  });

  it('测试文件不参与扫描（由 CLI 的文件过滤决定）', () => {
    // 与 check-network-registry.mjs 中的 isTestFile 保持同一判定
    expect(/\.(spec|test)\.[mc]?[jt]sx?$/.test('src/utils/artworkCache.spec.ts')).toBe(true);
    expect(/\.(spec|test)\.[mc]?[jt]sx?$/.test('src/utils/artworkCache.ts')).toBe(false);
  });
});

describe('清单悬空条目', () => {
  it('§1 登记了、代码里从不出现的主机以告警形式暴露', () => {
    const files = [{ content: 'let a = "https://lrclib.net/api/get";' }];
    const dangling = danglingHosts(registry, files);
    // only-in-table.test 属保留域，不进告警
    expect(dangling.map((d) => d.host).sort()).toEqual(['api.github.com', 'itunes.apple.com', 'music.163.com']);
  });
});

function a_violations_of(result) {
  return result.violations
    .filter((v) => v.type === 'missing-behavior-id')
    .map((v) => ({ line: v.line }));
}
