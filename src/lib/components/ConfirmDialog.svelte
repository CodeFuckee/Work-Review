<script lang="ts">
  import {
    confirmDialog,
    resolveConfirm,
    type ConfirmTone,
  } from '$lib/stores/confirm.ts';
  import Dialog, { Title, Content, Actions } from '@smui/dialog';
  import Button from '@smui/button';
  // bare.css 统一由 src/smui.css 相对路径引入（@smui 包 exports 不含 bare.css，直连会让 Vite build 失败）

  $: dialogState = $confirmDialog;
  // error 语气使用错误色主按钮，info/warning 用标准主按钮
  $: isDanger = (dialogState?.tone ?? 'info') === ('error' as ConfirmTone);
  // 受控 open：scrim 点击/Esc 关闭等同取消（resolveConfirm 对空队列是 no-op，重复调用安全）
  let dialogOpen = false;
  let shownId = -1;
  $: if (dialogState && shownId !== dialogState.id) {
    shownId = dialogState.id;
    dialogOpen = true;
  }
  $: if (!dialogState && dialogOpen) {
    dialogOpen = false;
  }
  $: if (!dialogOpen && shownId > 0) {
    resolveConfirm(false);
  }

  function decide(result: boolean): void {
    shownId = -1;
    dialogOpen = false;
    resolveConfirm(result);
  }

  // Escape = 取消（Carbon Modal 的键盘语义对齐）
  function handleKeydown(event: KeyboardEvent): void {
    if (dialogState && event.key === 'Escape') decide(false);
  }
</script>

<svelte:window on:keydown={handleKeydown} />

{#if dialogState}
  <Dialog bind:open={dialogOpen} aria-labelledby="confirm-dialog-title">
    <Title id="confirm-dialog-title" class="confirm-dialog-title">{dialogState.title}</Title>
    <Content>
      <p class="confirm-dialog-message">{dialogState.message}</p>
    </Content>
    <Actions>
      <Button class="confirm-dialog-btn" onclick={() => decide(false)}>
        {dialogState.cancelText}
      </Button>
      <Button
        class="confirm-dialog-btn{isDanger ? ' confirm-dialog-danger' : ''}"
        variant="raised"
        onclick={() => decide(true)}
      >
        {dialogState.confirmText}
      </Button>
    </Actions>
  </Dialog>
{/if}

<style>
  .confirm-dialog-title {
    font-size: 1.125rem;
    font-weight: 500;
  }

  .confirm-dialog-message {
    white-space: pre-line;
    color: var(--md-theme-on-surface-medium);
  }

  .confirm-dialog-btn :global(.mdc-button__label) {
    font-size: 0.875rem;
  }

  /* error 语气：容器与文字换错误色（覆盖 raised 主按钮的容器令牌） */
  .confirm-dialog-btn.confirm-dialog-danger :global(.mdc-button) {
    --mdc-protected-button-container-color: var(--md-theme-error);
    --mdc-protected-button-label-text-color: var(--md-theme-on-error);
  }
</style>
