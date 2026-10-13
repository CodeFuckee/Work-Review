<script lang="ts">
  // Material Symbols 图标（ligature 字体，@fontsource 本地打包，CSP 无需放宽）。
  // name 为 Material Symbols 图标名（如 home / settings / schedule），直接作为字形文本。
  export let name: string;
  export let size = 20;
</script>

<!-- svelte-ignore a11y-no-static-element-interactions -->
<span class="material-symbols-outlined md-icon" style="font-size:{size}px" aria-hidden="true">{name}</span>

<style>
  .md-icon {
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
