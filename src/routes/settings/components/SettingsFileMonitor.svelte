<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { open as openDialog } from '@tauri-apps/plugin-dialog';
  import { t } from '$lib/i18n/index.ts';
  import Toggle from 'carbon-components-svelte/src/Toggle/Toggle.svelte';

  interface FileMonitorSettings {
    enabled: boolean;
    folders: string[] | null;
    excludes: string[] | null;
  }

  export let config: { file_monitor?: FileMonitorSettings | null };

  const dispatch = createEventDispatcher<{ change: typeof config }>();

  function fileMonitor(): FileMonitorSettings {
    if (!config.file_monitor) {
      config.file_monitor = {
        enabled: false,
        folders: [],
        excludes: [],
      };
    }
    return config.file_monitor;
  }

  function handleChange() {
    dispatch('change', config);
  }

  function toggleEnabled() {
    fileMonitor().enabled = !fileMonitor().enabled;
    handleChange();
  }

  function foldersText(): string {
    return (fileMonitor().folders ?? []).join('\n');
  }

  function onFoldersInput(event: Event) {
    const value = (event.currentTarget as HTMLTextAreaElement).value;
    const lines = value
      .split('\n')
      .map((line) => line.trim())
      .filter((line) => line !== '');
    fileMonitor().folders = lines;
    handleChange();
  }

  async function pickFolder() {
    const picked = await openDialog({
      directory: true,
      multiple: true,
      title: t('settingsFileMonitor.pickFolderTitle'),
    });
    const folders = fileMonitor().folders ?? [];
    let changed = false;
    for (const item of Array.isArray(picked) ? picked : picked ? [picked] : []) {
      if (!folders.some((f) => f === item)) {
        folders.push(item);
        changed = true;
      }
    }
    if (changed) {
      fileMonitor().folders = folders;
      handleChange();
    }
  }

  function excludesText(): string {
    return (fileMonitor().excludes ?? []).join('\n');
  }

  function onExcludesInput(event: Event) {
    const value = (event.currentTarget as HTMLTextAreaElement).value;
    const lines = value
      .split(/[\n,]/)
      .map((line) => line.trim())
      .filter((line) => line !== '');
    fileMonitor().excludes = lines;
    handleChange();
  }
</script>

<div class="space-y-4">
  <div class="settings-row">
    <div class="min-w-0">
      <p class="settings-text">{t('settingsFileMonitor.enabled')}</p>
      <p class="settings-muted mt-1">{t('settingsFileMonitor.enabledDesc')}</p>
    </div>
    <Toggle size="sm" toggled={fileMonitor().enabled} labelText={t('settingsFileMonitor.enabled')} hideLabel={true} on:toggle={toggleEnabled} />
  </div>

  <div class="settings-field">
    <label class="settings-label" for="fm-folders">{t('settingsFileMonitor.folders')}</label>
    <textarea
      id="fm-folders"
      rows="4"
      class="w-full rounded-lg border border-slate-200 bg-white px-3 py-2 font-mono text-xs text-slate-900 focus:outline-none focus:ring-2 focus:ring-indigo-400 dark:border-[#2d3843] dark:bg-[#10161d] dark:text-[#e6edf3]"
      placeholder={t('settingsFileMonitor.foldersHint')}
      value={foldersText()}
      on:input={onFoldersInput}
      spellcheck="false"
    ></textarea>
    <div class="flex items-center justify-between gap-2">
      <p class="settings-muted">{t('settingsFileMonitor.foldersHint')}</p>
      <button
        type="button"
        class="page-control-btn min-h-8 shrink-0 px-3"
        on:click={pickFolder}
      >
        {t('settingsFileMonitor.pickFolder')}
      </button>
    </div>
  </div>

  <div class="settings-field">
    <label class="settings-label" for="fm-excludes">{t('settingsFileMonitor.excludes')}</label>
    <textarea
      id="fm-excludes"
      rows="4"
      class="w-full rounded-lg border border-slate-200 bg-white px-3 py-2 font-mono text-xs text-slate-900 focus:outline-none focus:ring-2 focus:ring-indigo-400 dark:border-[#2d3843] dark:bg-[#10161d] dark:text-[#e6edf3]"
      placeholder={t('settingsFileMonitor.excludesHint')}
      value={excludesText()}
      on:input={onExcludesInput}
      spellcheck="false"
    ></textarea>
    <p class="settings-muted mt-1">{t('settingsFileMonitor.excludesHint')}</p>
  </div>
</div>
