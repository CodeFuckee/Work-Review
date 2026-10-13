<script lang="ts">
  // Material Symbols 图标（ligature 字体，@fontsource 本地打包，CSP 无需放宽）。
  // name 为 Material Symbols 图标名（如 home / settings / schedule），直接作为字形文本。
  export let name: string;
  export let size = 20;
</script>

<!-- svelte-ignore a11y-no-static-element-interactions -->
<span class="material-symbols-outlined md-icon" style="font-size:{size}px" aria-hidden="true">{name}</span>

<style>
  /* fontsource index.css 只有 @font-face，图标类需自带 font-family 才能触发 ligature 字形 */
  .md-icon {
    font-family: 'Material Symbols Outlined';
    font-weight: normal;
    font-style: normal;
    letter-spacing: normal;
    text-transform: none;
    white-space: nowrap;
    word-wrap: normal;
    direction: ltr;
    font-feature-settings: 'liga';
    -webkit-font-smoothing: antialiased;
    line-height: 1;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    user-select: none;
    font-variation-settings:
      'FILL' 0,
      'wght' 400,
      'GRAD' 0,
      'opsz' 24;
  }
</style>
