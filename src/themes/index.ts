import type { AppTheme } from './types';
import SimpleLayout from './simple/index.vue';
import AdvancedLayout from './advanced/index.vue';
import TeLayout from './te/index.vue';
import ModernLayout from './modern/index.vue';

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
    name: '高级沉浸 (Advanced)',
    components: {
      Layout: AdvancedLayout,
    }
  },
  'theme-te': {
    id: 'theme-te',
    name: 'TE 工业风 (Teenage Engineering)',
    components: {
      Layout: TeLayout,
    }
  },
  'theme-modern': {
    id: 'theme-modern',
    name: '现代优雅 (Modern)',
    components: {
      Layout: ModernLayout,
    }
  }
};
