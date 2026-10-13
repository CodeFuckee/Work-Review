<script lang="ts">
  import { toast, clearToast, type ToastType } from '$lib/stores/toast.ts';
  import MdIcon from './MdIcon.svelte';

  // MD snackbar 形态：底部居中、按类型着图标（store API 与 Carbon 版一致）
  $: toastState = $toast;
  $: kind = (toastState?.type ?? 'info') as ToastType;
  $: kindIcon = kind === 'success' ? 'check_circle' : kind === 'error' ? 'error' : kind === 'warning' ? 'warning' : 'info';
  $: kindColor = kind === 'success' ? '#4caf50' : kind === 'error' ? '#cf6679' : kind === 'warning' ? '#ffb74d' : '#90caf9';
</script>

{#if toastState}
  <div
    class="pointer-events-none fixed inset-x-0 bottom-6 z-[210] flex justify-center px-4"
    role="status"
    aria-live="polite"
  >
    <div class="md-snackbar pointer-events-auto flex max-w-xl items-center gap-3">
      <MdIcon name={kindIcon} size={20} />
      <span class="md-snackbar-label" style="color:{kindColor}">{toastState.message}</span>
      <button class="md-snackbar-close" onclick={clearToast} aria-label="close">
        <MdIcon name="close" size={18} />
      </button>
    </div>
  </div>
{/if}

<style>
  .md-snackbar {
    padding: 0.875rem 1rem;
    border-radius: var(--radius-md);
    background: var(--mdc-snackbar-container-color, #333333);
    box-shadow:
      0 3px 5px -1px rgba(0, 0, 0, 0.2),
      0 6px 10px 0 rgba(0, 0, 0, 0.14),
      0 1px 18px 0 rgba(0, 0, 0, 0.12);
  }

  .md-snackbar-label {
    font-size: 0.875rem;
    line-height: 1.4;
    color: var(--mdc-snackbar-label-text-color, rgba(255, 255, 255, 0.95));
  }

  .md-snackbar-close {
    display: inline-flex;
    border: none;
    border-radius: 50%;
    background: transparent;
    color: var(--mdc-snackbar-label-text-color, rgba(255, 255, 255, 0.7));
    cursor: pointer;
    padding: 0.25rem;
  }

  .md-snackbar-close:hover {
    background: rgba(255, 255, 255, 0.12);
  }
</style>
