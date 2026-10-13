<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { createEventDispatcher } from 'svelte';
  import { t } from '../../lib/i18n/index.ts';
  import Select, { Option } from '@smui/select';

  // 「今日项目」归因卡：当日 Plane 项目清单 = 归因聚合(auto) − 当日屏蔽(removed)
  // ∪ 手动添加(added)。人工调整只影响本卡展示，不联动时间线行徽标（行级归因
  // 仍由算法按日现算）；未配置 Plane（下拉源空且无归因）时整卡隐藏。

  export let date: string;
  export let attributions: Array<{
    project_name: string;
  }> = [];
  // 手动归因状态（由父组件驱动；error=true 时展示轻量失败提示，几秒后自动消失）
  export let recomputing = false;
  export let recomputeError = false;
  // Plane 已配置（来自 get_config.plane.url）：配置了就显示卡——空态卡 +
  // 手动归因按钮仍可达（手动触发归因最常用的时刻恰是归因还没算出来时；
  // 若再要求项目列表非空，拉取闪断会把按钮一起藏掉且无重试）
  export let planeConfigured = false;

  const dispatch = createEventDispatcher<{ changed: void; recompute: void }>();

  let adjustments: Array<{ project_name: string; removed: boolean }> = [];
  let planeProjects: string[] = [];
  let loading = true;
  // 行内编辑状态：null = 关闭；{ mode: 'add' } 或 { mode: 'replace', from: string }
  let editor: { mode: 'add' } | { mode: 'replace'; from: string } | null = null;
  let editorSelection = '';
  let editorBusy = false;

  // 归因聚合出的项目（auto 来源）
  $: autoProjects = Array.from(
    new Set(
      attributions
        .map((attr) => attr.project_name.trim())
        .filter((name) => name.length > 0),
    ),
  ).sort();

  $: removedNames = new Set(adjustments.filter((a) => a.removed).map((a) => a.project_name));
  $: addedNames = new Set(adjustments.filter((a) => !a.removed).map((a) => a.project_name));

  // 最终清单：auto 未屏蔽 ∪ 手动添加（稳定排序）
  $: todayProjects = Array.from(new Set([...autoProjects.filter((p) => !removedNames.has(p)), ...addedNames])).sort();

  // 可见性以「已配置」为准；未配置（无 url）时才整卡隐藏
  $: cardVisible = planeConfigured || planeProjects.length > 0 || todayProjects.length > 0;

  // 下拉可选项：Plane 项目 − 已在清单中的
  $: selectableProjects = planeProjects.filter((name) => !todayProjects.includes(name));

  // invoke 失败与 resolve null（命令缺失/mock 表外）都归一为空数组——
  // null 流入派生表达式会抛 TypeError 破坏整页渲染
  async function loadAdjustments(): Promise<void> {
    try {
      adjustments = (await invoke('get_daily_project_adjustments', { date })) ?? [];
    } catch {
      adjustments = [];
    }
  }

  async function loadPlaneProjects(): Promise<void> {
    try {
      planeProjects = (await invoke<string[]>('list_plane_projects')) ?? [];
    } catch {
      planeProjects = [];
    }
  }

  onMount(async () => {
    await Promise.all([loadAdjustments(), loadPlaneProjects()]);
    loading = false;
  });

  // 切日期重载调整项（归因由父组件传入会自动更新）
  $: if (date) {
    void reloadForDate(date);
  }

  let lastLoadedDate = '';
  async function reloadForDate(next: string): Promise<void> {
    if (next === lastLoadedDate) return;
    lastLoadedDate = next;
    editor = null;
    await loadAdjustments();
  }

  async function applyAdjustment(projectName: string, removed: boolean): Promise<void> {
    if (editorBusy) return;
    editorBusy = true;
    try {
      await invoke('set_daily_project_adjustment', {
        date,
        projectName,
        removed,
      });
      await loadAdjustments();
      dispatch('changed');
    } catch (e) {
      console.warn('今日项目调整失败:', e);
    } finally {
      editorBusy = false;
    }
  }

  function openAddEditor(): void {
    editor = { mode: 'add' };
    editorSelection = selectableProjects[0] ?? '';
  }

  function openReplaceEditor(from: string): void {
    editor = { mode: 'replace', from };
    editorSelection = selectableProjects[0] ?? '';
  }

  async function confirmEditor(): Promise<void> {
    const target = editorSelection.trim();
    if (!target || !editor) return;
    if (editor.mode === 'replace') {
      // 替换 = 删旧 + 加新（两次 upsert，均轻量）
      await applyAdjustment(editor.from, true);
    }
    await applyAdjustment(target, false);
    editor = null;
  }

  async function removeProject(name: string): Promise<void> {
    await applyAdjustment(name, true);
  }

  function projectSourceLabel(name: string): string {
    if (addedNames.has(name)) return 'manual';
    return 'auto';
  }

  // 失败提示自动消失（手动点击的轻量反馈，不打断操作）
  let errorVisible = false;
  $: if (recomputeError) {
    errorVisible = true;
    setTimeout(() => (errorVisible = false), 4000);
  }

  onDestroy(() => {
    // 无常驻资源
  });
</script>

{#if !loading && cardVisible}
  <div class="page-card timeline-projects-card">
    <div class="timeline-projects-head">
      <h3 class="timeline-projects-title">{t('timeline.projects.title')}</h3>
      <button
        type="button"
        class="timeline-projects-recompute-btn"
        title={t('timeline.projects.recomputeTitle')}
        aria-label={t('timeline.projects.recomputeTitle')}
        disabled={recomputing}
        onclick={() => dispatch('recompute')}
      >
        {#if recomputing}
          <span class="timeline-projects-recompute-spinner" aria-hidden="true"></span>
        {:else}
          <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
          </svg>
        {/if}
      </button>
      {#if errorVisible}
        <span class="timeline-projects-error">{t('timeline.projects.recomputeFailed')}</span>
      {/if}
      {#if todayProjects.length === 0}
        <span class="timeline-projects-empty">{t('timeline.projects.empty')}</span>
      {/if}
    </div>
    <div class="timeline-projects-list">
      {#each todayProjects as name (name)}
        <span class="timeline-projects-chip" data-source={projectSourceLabel(name)}>
          <span class="timeline-projects-chip-name" title={name}>{name}</span>
          <button
            type="button"
            class="timeline-projects-chip-btn"
            title={t('timeline.projects.editLabel')}
            aria-label={t('timeline.projects.editLabel') + ' ' + name}
            onclick={() => openReplaceEditor(name)}
          >
            <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15.232 5.232l3.536 3.536m-2.036-5.036a2.5 2.5 0 113.536 3.536L6.5 21.036H3v-3.572L16.732 3.732z" />
            </svg>
          </button>
          <button
            type="button"
            class="timeline-projects-chip-btn"
            title={t('timeline.projects.removeLabel')}
            aria-label={t('timeline.projects.removeLabel') + ' ' + name}
            onclick={() => removeProject(name)}
          >
            <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>
        </span>
      {/each}

      {#if selectableProjects.length > 0 && !editor}
        <button
          type="button"
          class="timeline-projects-add-btn"
          onclick={openAddEditor}
        >
          + {t('timeline.projects.addLabel')}
        </button>
      {/if}

      {#if editor}
        <span class="timeline-projects-editor">
          {#if editor.mode === 'replace'}
            <span class="timeline-projects-editor-from" title={editor.from}>{editor.from}</span>
            <span class="timeline-projects-editor-arrow" aria-hidden="true">→</span>
          {/if}
          <Select
            class="timeline-projects-select"
            noLabel={true}
            anchor$aria-label={t('timeline.projectsEditorTitle')}
            disabled={editorBusy}
            bind:value={editorSelection}
          >
            {#each selectableProjects as name (name)}
              <Option value={name}>{name}</Option>
            {/each}
          </Select>
          <button
            type="button"
            class="timeline-projects-editor-confirm"
            disabled={editorBusy || !editorSelection}
            onclick={confirmEditor}
          >
            ✓
          </button>
          <button
            type="button"
            class="timeline-projects-editor-cancel"
            disabled={editorBusy}
            onclick={() => (editor = null)}
          >
            ✕
          </button>
        </span>
      {/if}
    </div>
  </div>
{/if}

<style>
  .timeline-projects-card {
    padding: 0.75rem 1rem;
    margin-bottom: 1rem;
  }

  .timeline-projects-head {
    display: flex;
    align-items: baseline;
    gap: 0.75rem;
    margin-bottom: 0.5rem;
  }

  .timeline-projects-title {
    font-size: 0.9rem;
    font-weight: 600;
    color: #1f2937;
  }

  :global(.dark) .timeline-projects-title {
    color: #c9d1d9;
  }

  .timeline-projects-recompute-btn {
    display: inline-flex;
    align-items: center;
    padding: 0.2rem;
    border-radius: var(--radius-sm);
    color: #94a3b8;
    transition: color 0.15s ease;
  }

  .timeline-projects-recompute-btn:hover:not(:disabled) {
    color: rgb(37 99 235);
  }

  .timeline-projects-recompute-btn:disabled {
    cursor: wait;
    opacity: 0.6;
  }

  .timeline-projects-recompute-spinner {
    width: 0.85rem;
    height: 0.85rem;
    border: 2px solid currentColor;
    border-top-color: transparent;
    border-radius: var(--radius-full);
    animation: timeline-projects-spin 0.8s linear infinite;
  }

  @keyframes timeline-projects-spin {
    to {
      transform: rotate(360deg);
    }
  }

  .timeline-projects-error {
    font-size: 0.78rem;
    color: #e11d48;
  }

  .timeline-projects-empty {
    font-size: 0.8rem;
    color: #94a3b8;
  }

  .timeline-projects-list {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.5rem;
  }

  .timeline-projects-chip {
    display: inline-flex;
    align-items: center;
    gap: 0.3rem;
    padding: 0.2rem 0.3rem 0.2rem 0.6rem;
    border-radius: var(--radius-md);
    font-size: 0.8rem;
    line-height: 1.4;
    white-space: nowrap;
    color: rgb(37 99 235);
    background: rgb(37 99 235 / 0.1);
  }

  :global(.dark) .timeline-projects-chip {
    color: rgb(147 197 253);
    background: rgb(37 99 235 / 0.18);
  }

  .timeline-projects-chip[data-source='manual'] {
    color: rgb(139 92 246);
    background: rgb(139 92 246 / 0.1);
  }

  :global(.dark) .timeline-projects-chip[data-source='manual'] {
    color: rgb(196 181 253);
    background: rgb(139 92 246 / 0.18);
  }

  .timeline-projects-chip-name {
    max-width: 18rem;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .timeline-projects-chip-btn {
    display: inline-flex;
    align-items: center;
    padding: 0.15rem;
    border-radius: var(--radius-sm);
    color: inherit;
    opacity: 0.55;
    transition: opacity 0.15s ease;
  }

  .timeline-projects-chip-btn:hover {
    opacity: 1;
  }

  .timeline-projects-add-btn {
    padding: 0.2rem 0.6rem;
    border: 1px dashed rgb(148 163 184 / 0.6);
    border-radius: var(--radius-md);
    font-size: 0.8rem;
    color: #64748b;
    transition: color 0.15s ease, border-color 0.15s ease;
  }

  .timeline-projects-add-btn:hover {
    color: rgb(37 99 235);
    border-color: rgb(37 99 235 / 0.5);
  }

  .timeline-projects-editor {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
  }

  .timeline-projects-editor-from {
    max-width: 10rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 0.8rem;
    color: #94a3b8;
    text-decoration: line-through;
  }

  .timeline-projects-editor-arrow {
    color: #94a3b8;
    font-size: 0.8rem;
  }

  /* SMUI Select（MDC standard 形态）：压缩成行内小尺寸下拉 */
  .timeline-projects-select {
    max-width: 14rem;
    font-size: 0.8rem;
    color: inherit;
  }

  .timeline-projects-select :global(.mdc-select__anchor) {
    height: 2rem;
    padding-inline: 0.4rem;
  }

  .timeline-projects-select :global(.mdc-select__selected-text) {
    padding: 0;
    font-size: 0.8rem;
    line-height: 2rem;
  }

  .timeline-projects-select :global(.mdc-select__dropdown-icon) {
    width: 16px;
    height: 16px;
  }

  .timeline-projects-editor-confirm,
  .timeline-projects-editor-cancel {
    padding: 0.15rem 0.4rem;
    border-radius: var(--radius-sm);
    font-size: 0.8rem;
    color: inherit;
    opacity: 0.7;
  }

  .timeline-projects-editor-confirm:hover:not(:disabled),
  .timeline-projects-editor-cancel:hover:not(:disabled) {
    opacity: 1;
  }

  .timeline-projects-editor-confirm:disabled,
  .timeline-projects-editor-cancel:disabled {
    opacity: 0.3;
    cursor: not-allowed;
  }
</style>
