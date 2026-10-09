<script lang="ts">
  import {
    confirmDialog,
    resolveConfirm,
    type ConfirmTone,
  } from '$lib/stores/confirm.ts';
  import Modal from 'carbon-components-svelte/src/Modal/Modal.svelte';

  $: dialogState = $confirmDialog;
  // error 语气使用 Carbon danger 变体（红色主按钮），info/warning 用标准主按钮
  $: isDanger = (dialogState?.tone ?? 'info') === ('error' as ConfirmTone);
</script>

{#if dialogState}
  <Modal
    open={true}
    danger={isDanger}
    alert={true}
    modalHeading={dialogState.title}
    modalAriaLabel={dialogState.title}
    primaryButtonText={dialogState.confirmText}
    secondaryButtonText={dialogState.cancelText}
    shouldSubmitOnEnter={false}
    on:submit={() => resolveConfirm(true)}
    on:close={() => resolveConfirm(false)}
  >
    <p class="confirm-dialog-message">{dialogState.message}</p>
  </Modal>
{/if}

<style>
  .confirm-dialog-message {
    white-space: pre-line;
    color: var(--cds-text-secondary);
  }
</style>
