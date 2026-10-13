// 样式加载顺序：Material 令牌层（MDC 变量明暗两套）→ SMUI 组件预编译样式（bare.css 集中引入）→
// Roboto/Noto 字体 → 应用全局样式（字体链/覆盖最后生效）
import './md-theme.css';
import './smui.css';
import '@fontsource/roboto/400.css';
import '@fontsource/roboto/500.css';
import '@fontsource/roboto/700.css';
import '@fontsource/noto-sans-sc/400.css';
import '@fontsource/noto-sans-sc/500.css';
import '@fontsource/noto-sans-sc/700.css';
import '@fontsource/noto-sans-arabic/400.css';
import '@fontsource/noto-sans-arabic/500.css';
import '@fontsource/material-symbols-outlined';
import './app.css';
import { mount } from 'svelte';
import App from './App.svelte';

const target = document.getElementById('app');

if (!target) {
  throw new Error('找不到应用挂载节点 #app');
}

// Svelte 5：mount API（兼容 Svelte 4 编译的子组件，如 svelte-spa-router）
const app = mount(App, { target });

export default app;
