import type { AppTheme } from './types';
import SimpleLayout from './simple/index.vue';
import AdvancedLayout from './advanced/index.vue';

export const themes: Record<string, AppTheme> = {
  'theme-simple': {
    id: 'theme-simple',
    name: '极简模板 (Simple)',
    components: {
      Layout: SimpleLayout,
    }
  },
  'theme-advanced': {
    id: 'theme-advanced',
    name: '高级模板 (Advanced)',
    components: {
      Layout: AdvancedLayout,
    }
  }
};
