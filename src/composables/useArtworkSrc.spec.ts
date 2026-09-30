import { describe, expect, it, vi } from 'vitest';
import { ref } from 'vue';
import { useArtworkSrc } from './useArtworkSrc';

describe('单一封面 URL 通道', () => {
  it('快速切换封面不会创建额外 fetch、Base64 或异步预取队列', () => {
    const fetchSpy = vi.spyOn(globalThis, 'fetch');
    try {
      const id = ref<number | null>(1);
      const thumb = useArtworkSrc(() => id.value);
      const full = useArtworkSrc(() => id.value, 'full');
      expect(thumb.value).toMatch(/\/artwork\/1\?size=thumb$/);
      expect(full.value).toMatch(/\/artwork\/1$/);
      for (let i = 2; i < 100; i++) id.value = i;
      expect(thumb.value).toMatch(/\/artwork\/99\?size=thumb$/);
      id.value = null;
      expect(thumb.value).toBe('');
      expect(fetchSpy).not.toHaveBeenCalled();
    } finally { fetchSpy.mockRestore(); }
  });
});
