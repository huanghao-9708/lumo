import type { Component } from 'vue';

export interface AppTheme {
  id: string;
  name: string;
  components: {
    Layout: Component;
  };
}
