<script lang="ts">
  import { toast, clearToast, type ToastType } from '$lib/stores/toast.ts';
  import ToastNotification from 'carbon-components-svelte/src/Notification/ToastNotification.svelte';

  // Carbon kind 与内部 ToastType 一一对应（success/error/warning/info）
  $: toastState = $toast;
  $: kind = (toastState?.type ?? 'info') as ToastType;
</script>

{#if toastState}
  <div
    class="pointer-events-none fixed inset-x-0 bottom-6 z-[210] flex justify-center px-4"
    role="status"
    aria-live="polite"
  >
    <div class="pointer-events-auto max-w-xl">
      <ToastNotification
        {kind}
        title=""
        subtitle={toastState.message}
        timeout={0}
        on:close={() => clearToast()}
      />
    </div>
  </div>
{/if}
