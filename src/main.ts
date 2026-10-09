// 样式加载顺序：Carbon 全量主题（含 [theme=gX] 属性作用域变体）→ IBM Plex 字体 → 应用全局样式（字体链/覆盖最后生效）
import 'carbon-components-svelte/css/all.css';
import '@ibm/plex-sans/css/ibm-plex-sans-all.css';
import '@ibm/plex-sans-arabic/css/ibm-plex-sans-arabic-all.css';
import './app.css';
import App from './App.svelte';

const target = document.getElementById('app');

if (!target) {
  throw new Error('找不到应用挂载节点 #app');
}

const app = new App({ target });

export default app;
