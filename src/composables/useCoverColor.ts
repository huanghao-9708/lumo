import { ref, watch, type Ref } from 'vue';

/**
 * useCoverColor —— 从封面图实时提取主色 / 辅色（用于沉浸式播放页背景渲染）。
 *
 * 工作机制：
 * 1. 把封面绘到 64×64 的缩略 canvas（缩小后取色更快也更稳）
 * 2. 遍历像素做简单的「色相分桶 + 出现频次加权 + 饱和度偏好」统计
 *    → 取出现最多且较鲜艳的一桶作为 primary（主色），
 *      再取另一桶（差异最大的暗色 / 浅色）作为 secondary（辅色）。
 * 3. 仅在 src 变化时跑一次，零持续开销。
 *
 * CORS 注意：
 *   useArtworkSrc 可能返回 data URL（内存缓存命中，无污染），
 *   也可能返回 lumo:// 或 http://lumo.localhost 自定义协议 URL。
 *   后者会让 canvas 被「污染」（tainted），getImageData 抛 SecurityError。
 *   因此整个流程包在 try/catch 里，失败时 ready 保持 false、颜色返回空串，
 *   调用方据此回落到 Track.coverColor 渐变桩（见 NowPlayingImmersive）。
 *
 * 返回：
 *   primary   —— 主色，'rgb(r, g, b)' 字符串；不可用时为 ''
 *   secondary —— 辅色，同上
 *   ready     —— 本轮取色是否成功
 */
export interface CoverColorResult {
  primary: Ref<string>;
  secondary: Ref<string>;
  ready: Ref<boolean>;
}

const SIZE = 64; // 缩略尺寸：64×64 = 4096 个采样像素，足够稳

export function useCoverColor(srcGetter: () => string | null | undefined): CoverColorResult {
  const primary = ref('');
  const secondary = ref('');
  const ready = ref(false);

  watch(
    srcGetter,
    (src) => {
      if (!src) {
        primary.value = '';
        secondary.value = '';
        ready.value = false;
        return;
      }
      extract(src).then(
        (res) => {
          if (!res) {
            primary.value = '';
            secondary.value = '';
            ready.value = false;
            return;
          }
          primary.value = res.primary;
          secondary.value = res.secondary;
          ready.value = true;
        },
        () => {
          primary.value = '';
          secondary.value = '';
          ready.value = false;
        },
      );
    },
    { immediate: true },
  );

  return { primary, secondary, ready };
}

interface Extracted {
  primary: string;
  secondary: string;
}

async function extract(src: string): Promise<Extracted | null> {
  // SSR / 测试环境兜底
  if (typeof document === 'undefined') return null;

  const img = await loadImage(src);
  if (!img) return null;

  const canvas = document.createElement('canvas');
  canvas.width = SIZE;
  canvas.height = SIZE;
  const ctx = canvas.getContext('2d', { willReadFrequently: true });
  if (!ctx) return null;

  try {
    ctx.drawImage(img, 0, 0, SIZE, SIZE);
    // data URL 不污染；自定义协议 URL 会在此抛 SecurityError
    const { data } = ctx.getImageData(0, 0, SIZE, SIZE);
    return pickColors(data);
  } catch {
    // canvas 被污染：取色不可用，回落到调用方的兜底色
    return null;
  }
}

function loadImage(src: string): Promise<HTMLImageElement | null> {
  return new Promise((resolve) => {
    const img = new Image();
    // data URL 直接可用；自定义协议同样走 <img> 加载（crossOrigin 关闭以避免改变行为）
    img.onload = () => resolve(img);
    img.onerror = () => resolve(null);
    img.src = src;
  });
}

/* ============ 简易取色算法 ============
   把每个像素的 RGB 转成 HSL，按 H（色相）分桶统计：
   - 跳过接近黑/白/灰的低饱和像素（它们是「无彩色」，不该当主色）
   - 桶内累加出现次数与该像素的饱和度（越鲜艳权重越高）
   - primary = 权重最高的桶的均色
   - secondary = 与 primary 差异最大的另一个彩色桶均色；若只有一桶，则取 primary 的明度变体
   最后按亮度裁剪，避免主色过亮/过暗看不出。 */

interface Bucket {
  h: number; // 桶中心色相 0-360
  count: number;
  weight: number; // count × 平均饱和度
  rSum: number;
  gSum: number;
  bSum: number;
}

const BUCKET_STEP = 15; // 每 15° 一桶，共约 24 桶

function pickColors(data: Uint8ClampedArray): Extracted {
  const buckets = new Map<number, Bucket>();

  for (let i = 0; i < data.length; i += 4) {
    const r = data[i];
    const g = data[i + 1];
    const b = data[i + 2];
    const a = data[i + 3];
    if (a < 128) continue; // 透明像素跳过

    const { h, s, l } = rgbToHsl(r, g, b);
    // 过滤无彩色：饱和度太低 或 极亮/极暗
    if (s < 0.18 || l < 0.08 || l > 0.94) continue;

    const key = Math.floor(h / BUCKET_STEP) * BUCKET_STEP;
    const bucket = buckets.get(key);
    if (bucket) {
      bucket.count++;
      bucket.weight += s; // 鲜艳像素加权
      bucket.rSum += r;
      bucket.gSum += g;
      bucket.bSum += b;
    } else {
      buckets.set(key, {
        h: key + BUCKET_STEP / 2,
        count: 1,
        weight: s,
        rSum: r,
        gSum: g,
        bSum: b,
      });
    }
  }

  // 没有彩色像素（纯灰阶图）—— 直接用整体均色
  if (buckets.size === 0) {
    const c = averageColor(data);
    return { primary: c, secondary: c };
  }

  const sorted = [...buckets.values()].sort((a, b) => b.weight - a.weight);
  const top = sorted[0];
  const primary = bucketAvgRgb(top);

  // secondary：在剩余桶里找色相差最大的
  let secondary: string;
  if (sorted.length > 1) {
    let maxDelta = -1;
    let chosen = sorted[1];
    for (let i = 1; i < sorted.length; i++) {
      const delta = hueDistance(top.h, sorted[i].h);
      if (delta > maxDelta) {
        maxDelta = delta;
        chosen = sorted[i];
      }
    }
    secondary = bucketAvgRgb(chosen);
  } else {
    // 只有一桶彩色：退化成把 primary 调暗一档作为辅色，给背景做晕影
    secondary = shiftLightness(top, false);
  }

  return { primary, secondary };
}

function bucketAvgRgb(bucket: Bucket): string {
  const r = Math.round(bucket.rSum / bucket.count);
  const g = Math.round(bucket.gSum / bucket.count);
  const b = Math.round(bucket.bSum / bucket.count);
  return `rgb(${r}, ${g}, ${b})`;
}

function averageColor(data: Uint8ClampedArray): string {
  let r = 0,
    g = 0,
    b = 0,
    n = 0;
  for (let i = 0; i < data.length; i += 4) {
    if (data[i + 3] < 128) continue;
    r += data[i];
    g += data[i + 1];
    b += data[i + 2];
    n++;
  }
  if (n === 0) return 'rgb(60,60,60)';
  return `rgb(${Math.round(r / n)}, ${Math.round(g / n)}, ${Math.round(b / n)})`;
}

/** secondary 兜底：把 primary 调暗一档，给背景做晕影 */
function shiftLightness(bucket: Bucket, brighter: boolean): string {
  const r = Math.round(bucket.rSum / bucket.count);
  const g = Math.round(bucket.gSum / bucket.count);
  const b = Math.round(bucket.bSum / bucket.count);
  const { h, s, l } = rgbToHsl(r, g, b);
  const nl = brighter ? Math.min(0.92, l + 0.2) : Math.max(0.1, l - 0.25);
  const [nr, ng, nb] = hslToRgb(h, s, nl);
  return `rgb(${nr}, ${ng}, ${nb})`;
}

function hueDistance(a: number, b: number): number {
  const d = Math.abs(a - b) % 360;
  return d > 180 ? 360 - d : d;
}

/* ---- 颜色空间换算 ---- */
function rgbToHsl(r: number, g: number, b: number): { h: number; s: number; l: number } {
  r /= 255;
  g /= 255;
  b /= 255;
  const max = Math.max(r, g, b);
  const min = Math.min(r, g, b);
  let h = 0;
  const l = (max + min) / 2;
  const d = max - min;
  const s = d === 0 ? 0 : d / (1 - Math.abs(2 * l - 1));
  if (d !== 0) {
    switch (max) {
      case r:
        h = ((g - b) / d) % 6;
        break;
      case g:
        h = (b - r) / d + 2;
        break;
      default:
        h = (r - g) / d + 4;
    }
    h *= 60;
    if (h < 0) h += 360;
  }
  return { h, s, l };
}

function hslToRgb(h: number, s: number, l: number): [number, number, number] {
  const c = (1 - Math.abs(2 * l - 1)) * s;
  const x = c * (1 - Math.abs(((h / 60) % 2) - 1));
  const m = l - c / 2;
  let r = 0,
    g = 0,
    b = 0;
  if (h < 60) [r, g, b] = [c, x, 0];
  else if (h < 120) [r, g, b] = [x, c, 0];
  else if (h < 180) [r, g, b] = [0, c, x];
  else if (h < 240) [r, g, b] = [0, x, c];
  else if (h < 300) [r, g, b] = [x, 0, c];
  else [r, g, b] = [c, 0, x];
  return [Math.round((r + m) * 255), Math.round((g + m) * 255), Math.round((b + m) * 255)];
}
