/**
 * 联网行为登记的纯规则（CR-008）——被 check-network-registry.mjs 使用，并被 vitest 直接断言。
 *
 * 为什么要单独抽出这个模块：门禁的价值全在判定逻辑，而判定逻辑必须能被正反两例测试钉住
 * （「已登记文件里新增域名 → 必须失败」「动态 URL 调用点缺行为 ID → 必须失败」）。
 * 留在 CLI 里就只能靠人工注入变异来验证，成本高到没人会做。
 *
 * 这里刻意不做任何 I/O：输入是「文档正文 + 文件列表」，输出是违规与告警，方便测试。
 */

/** 行为 ID 声明标记：`联网行为: §2A` / `联网行为: §2E §2F` / `联网行为: local` */
const MARKER = /联网行为[:：]\s*([^\n]*)/;
const MARKER_ID = /§2\s*([A-Z])\b/g;

/** 形如 `http(s)://host[:port]` 的目标；host 允许 IPv6 字面量 `[::1]`。 */
const URL_IN_TEXT = /https?:\/\/((?:\[[0-9A-Fa-f:.]+\])|[A-Za-z0-9](?:[A-Za-z0-9._-]*[A-Za-z0-9])?)(?::\d+)?/g;

/**
 * Rust：以 HTTP 客户端接收者发起请求的调用点，以及把远端地址绑进客户端的构造点。
 * 只认「名字里含 client」的接收者，避免 `pool.get()`、`map.get(k)`、`row.get(0)` 这类同名方法
 * 把门禁变成噪音源——门禁一旦误报，很快就会被后人整段关掉。
 * 构造点（`WebdavClient::new` / `reqwest::Client::builder()`）同样要声明：那才是「这次外联去哪」
 * 真正被决定的地方，比动词调用点更接近策略层。
 */
const RUST_CALL_SITE =
  /(?:\breqwest::(?:blocking::)?(?:get|post|put|delete|patch|head)\s*\()|(?:[A-Za-z0-9_]+\s*\.\s*)?[A-Za-z0-9_]*client[A-Za-z0-9_]*\s*\.\s*(?:get|post|put|delete|patch|head|request)\s*\(\s*(?:[^)\s]|$)|\b[A-Za-z0-9_]*[Cc]lient\s*::\s*builder\s*\(|\b[A-Za-z0-9_]*[Cc]lient\s*::\s*new\s*\(\s*(?:[^)\s]|$)/i;

/**
 * 前端：真正会出网的调用点。刻意不看「任意字符串字面量」，否则设置页的 placeholder 会全量误报。
 * 裸 `open(...)` 不列入——那是 Tauri dialog 插件的本地文件夹选择器，不出网。
 */
const FRONTEND_CALL_SITE =
  /(?:^|[^A-Za-z0-9_$])fetch\s*\(|\bwindow\s*\.\s*open\s*\(|\bnew\s+WebSocket\s*\(|\bnew\s+EventSource\s*\(|\bsendBeacon\s*\(|\baxios\s*[.[]|(?:location|document\.location)\s*\.\s*href\s*=/;

/** 本地回环 / 私网 / 保留域名：属于测试夹具与本机服务，不算外联。 */
const NON_OUTBOUND_HOSTS = new Set(['localhost', '0.0.0.0', '::1', 'example.com', 'example.org', 'example.net', 'example']);

export function isLoopbackHost(host) {
  const h = host.replace(/^\[(?:::)?\]/, '::1').toLowerCase();
  if (NON_OUTBOUND_HOSTS.has(h)) return true;
  if (h.startsWith('127.')) return true;
  // *.local / *.invalid / .test 是 mDNS 与测试保留域，不会真的走到公网
  if (/\.(local|invalid|test|example)$/.test(h)) return true;
  return false;
}

/** 一行是否参与编译/执行：跳过 `//`、`/*`、` * ` 续行与 `#` 开头的脚本注释 */
export function isCodeLine(line) {
  const t = line.trim();
  return !(t.startsWith('//') || t.startsWith('/*') || t.startsWith('*') || t.startsWith('#'));
}

export function extractHosts(text) {
  const hosts = [];
  for (const m of text.matchAll(URL_IN_TEXT)) hosts.push(m[1]);
  return hosts;
}

/**
 * 解析 NETWORK_BEHAVIOR.md 的 §1 总览表与 §2 明细标题，得到「行为 ID → 允许的主机集合」。
 *
 * 两个来源都要有：§1 说明这条外联存在且登记了主机，§2 说明它的触发方式与落地位置真的写清楚了。
 * 只有 §1 没有 §2（或反之）的 ID 视为不完整登记，不允许被调用点引用。
 */
export function parseRegistry(docText) {
  const rows = new Map();
  const lines = docText.split(/\r?\n/);
  // 只在 §1 / §2 小节内解析：文档别处的表格也可能出现 `| A | …` 开头的行。
  const sectionOf = (line) => /^##\s+(\d+)\./.exec(line)?.[1] ?? null;
  let current = null;
  const section1 = [];
  const section2 = [];
  for (const line of lines) {
    const s = sectionOf(line);
    if (s) current = s;
    if (current === '1') section1.push(line);
    else if (current === '2') section2.push(line);
  }

  for (const line of section1) {
    const m = /^\|\s*([A-Z])\s*\|(.*)$/.exec(line);
    if (!m) continue;
    const cells = line.split('|').slice(1, -1).map((c) => c.trim());
    if (cells.length < 3) continue;
    const [, , hosts] = cells;
    const parsed = new Set();
    for (const token of hosts.matchAll(/`([^`]+)`/g)) {
      const value = token[1].trim();
      // 必须含点才算主机：§1 里同一列还写着 `base_url` / `root_uri` 这类配置字段名
      if (/^[a-zA-Z0-9._[\]-]+$/.test(value) && value.includes('.')) parsed.add(value.toLowerCase());
    }
    const id = m[1];
    // 同一 ID 可能在文档里出现多次（§1 表 + 附录），主机取并集
    const existing = rows.get(id);
    if (existing) {
      for (const h of parsed) existing.hosts.add(h);
      existing.listed = true;
    } else {
      rows.set(id, { id, label: cells[1], hosts: parsed, listed: true, section: false });
    }
  }

  for (const line of section2) {
    const m = /^###\s+([A-Z])\.\s+/.exec(line);
    if (!m) continue;
    const row = rows.get(m[1]);
    if (row) row.section = true;
    else rows.set(m[1], { id: m[1], label: '', hosts: new Set(), listed: false, section: true });
  }

  const allHosts = new Map();
  for (const row of rows.values()) {
    for (const host of row.hosts) {
      if (!allHosts.has(host)) allHosts.set(host, []);
      allHosts.get(host).push(row.id);
    }
  }

  return { rows, allHosts };
}

/** 解析标记内容：`{ ids: ['E','F'], local: false }`；没有标记返回 null */
export function parseMarker(text) {
  const m = MARKER.exec(text);
  if (!m) return null;
  const rest = m[1];
  const ids = [...rest.matchAll(MARKER_ID)].map((x) => x[1]);
  return { ids, local: /(?:^|[\s,])local(?:$|[\s,])/i.test(rest) };
}

/**
 * 向上最多三行找声明注释。允许注释与调用点之间隔空行/注释行，但不允许穿过代码行——
 * 跨过代码行的"声明"归属不清，读者会在几十行后才发现它其实管的是另一个请求。
 * 每个标记只服务一个调用点：一个标记罩住三处请求正是旧版文件级门禁失效的形态。
 */
function findPrecedingMarker(physical, index, usedMarkers) {
  for (let back = 1; back <= 3 && index - back >= 0; back += 1) {
    const trimmed = physical[index - back].trim();
    if (trimmed === '') continue;
    const isComment = trimmed.startsWith('//') || trimmed.startsWith('#') || trimmed.startsWith('/*');
    if (!isComment) return null;
    const parsed = parseMarker(trimmed);
    if (!parsed) continue;
    // 最近的标记即使已被别的调用点用掉，也不再往上找：向上继承只允许一层
    if (usedMarkers.has(index - back)) return null;
    usedMarkers.add(index - back);
    return parsed;
  }
  return null;
}

/**
 * 把 Rust 常见的链式换行折叠成一条逻辑行。
 * `let resp = client` 换行 `.get(url)` 若按物理行判断，等于放过所有跨行调用点——
 * 这是旧版文件级门禁最容易被绕过的方式（写个换行就"看不见"了）。
 */
export function toLogicalLines(lines) {
  const out = [];
  lines.forEach((line, index) => {
    const previous = out[out.length - 1];
    const prevPhysical = previous ? lines[previous.end] : '';
    const continues =
      previous &&
      /^\s*\./.test(line) &&
      isCodeLine(prevPhysical) &&
      !/[;{}]$/.test(prevPhysical.trim());
    if (continues) {
      previous.text += ' ' + line.trim();
      previous.end = index;
      return;
    }
    out.push({ text: line, start: index, end: index });
  });
  return out;
}

/**
 * 单文件扫描。kind: 'rust' | 'frontend'
 * Rust 额外扫「任意代码行里的字面量主机」（R1），前端只扫调用点行（避免 placeholder 误报）。
 */
export function scanFile({ path, kind, content, registry }) {
  const violations = [];
  const declared = [];
  let physical = content.split(/\r?\n/);

  if (kind === 'rust') {
    const testStart = physical.findIndex((l) => /^#\[cfg\(test\)\]/.test(l));
    if (testStart >= 0) physical = physical.slice(0, testStart);
  }

  const callSiteRe = kind === 'rust' ? RUST_CALL_SITE : FRONTEND_CALL_SITE;
  const usedMarkers = new Set();
  // 从前往后遍历调用点：每个标记最多服务一个调用点，逼着「一处声明、一处使用」
  for (const logical of toLogicalLines(physical)) {
    const line = logical.text;
    const index = logical.start;
    if (!isCodeLine(line)) continue;
    const isCallSite = callSiteRe.test(line);
    if (kind === 'rust' || isCallSite) {
      for (const host of extractHosts(line)) {
        if (isLoopbackHost(host)) continue;
        if (registry.allHosts.has(host)) continue;
        violations.push({
          file: path,
          line: index + 1,
          type: 'undeclared-host',
          detail: `主机 ${host} 未登记在 NETWORK_BEHAVIOR.md §1`,
        });
      }
    }
    if (!isCallSite) continue;

    const marker = parseMarker(line) ?? findPrecedingMarker(physical, index, usedMarkers);

    if (!marker) {
      violations.push({
        file: path,
        line: index + 1,
        type: 'missing-behavior-id',
        detail: '动态调用点缺少 `联网行为: §2X`（或 `联网行为: local`）声明',
      });
      continue;
    }
    if (marker.local) {
      declared.push({ file: path, line: index + 1, ids: [], local: true });
      continue;
    }
    if (!marker.ids.length) {
      violations.push({
        file: path,
        line: index + 1,
        type: 'invalid-behavior-id',
        detail: `声明「${MARKER.exec(line)?.[1] ?? '联网行为'}」里无法解析出 §2X 形式的行为 ID`,
      });
      continue;
    }
    for (const id of marker.ids) {
      const row = registry.rows.get(id);
      if (!row || !row.listed || !row.section) {
        violations.push({
          file: path,
          line: index + 1,
          type: 'unknown-behavior-id',
          detail: `§2${id} 登记不完整（§1 总览行与 §2 明细小节必须同时存在）`,
        });
      }
    }
    const known = marker.ids.filter((id) => registry.rows.has(id));
    for (const host of extractHosts(line)) {
      if (isLoopbackHost(host)) continue;
      const owner = registry.allHosts.get(host);
      const ok = known.some((id) => registry.rows.get(id)?.hosts.has(host));
      if (owner && !ok) {
        violations.push({
          file: path,
          line: index + 1,
          type: 'host-row-mismatch',
          detail: `调用点声明为 §2${known.join('/§2')}，但主机 ${host} 登记在 §2${owner.join('/§2')}`,
        });
      }
    }
    declared.push({ file: path, line: index + 1, ids: known, local: false });
  }

  return { violations, declared };
}

/** §1 登记了字面量主机、但代码里从未出现——通常是响应内 CDN 或已删除的能力，只告警不失败 */
export function danglingHosts(registry, files) {
  const seen = new Set();
  for (const file of files) {
    for (const host of extractHosts(file.content)) seen.add(host.toLowerCase());
  }
  const dangling = [];
  for (const [host, ids] of registry.allHosts) {
    if (seen.has(host)) continue;
    if (isLoopbackHost(host)) continue;
    dangling.push({ host, ids });
  }
  return dangling;
}

export function formatViolation(v) {
  return `${v.file}:${v.line} [${v.type}] ${v.detail}`;
}
