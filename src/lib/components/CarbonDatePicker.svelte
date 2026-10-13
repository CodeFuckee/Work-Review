<script lang="ts">
  // MD DatePicker：flatpickr 直驱（去 Carbon 化），对外 API 与旧 LocalizedDatePicker 一致
  // （mode/value/startDate/endDate/localeCode/min/max/triggerClass + change 事件），
  // Overview/Timeline/Report 的调用点零改动。日期一律走 ISO（YYYY-MM-DD）字符串。
  import { createEventDispatcher, onMount, onDestroy } from 'svelte';
  import flatpickr from 'flatpickr';
  import type { Instance } from 'flatpickr/dist/types/instance';
  // l10n 为 CJS 默认导出（命名空间对象），不能用具名导入
  import l10n from 'flatpickr/dist/l10n/index.js';

  // 宽化为 string：调用点传入字面量时由内部收窄
  export let mode: 'single' | 'range' | string = 'single';
  export let localeCode = 'zh-CN';
  export let value = '';
  export let startDate = '';
  export let endDate = '';
  export let min = '';
  export let max = '';
  // API 兼容位：旧组件的触发器样式类（现注入 MD 输入框外观，调用点原样保留）
  export let triggerClass = '';

  const dispatch = createEventDispatcher<{ change: { value?: string; startDate?: string; endDate?: string } }>();

  let inputEl: HTMLInputElement;
  let fp: Instance | null = null;

  function flatpickrLocale(code: string): typeof l10n.zh {
    switch (code) {
      case 'zh-CN': return l10n.zh;
      case 'zh-TW': return l10n.zh_tw;
      case 'ar': return l10n.ar;
      default: return l10n.en;
    }
  }

  function toIso(date: Date | undefined | null): string {
    if (!date || Number.isNaN(date.getTime())) return '';
    const y = date.getFullYear();
    const m = String(date.getMonth() + 1).padStart(2, '0');
    const d = String(date.getDate()).padStart(2, '0');
    return `${y}-${m}-${d}`;
  }

  function fromIso(iso: string): Date | undefined {
    if (!iso) return undefined;
    const date = new Date(`${iso}T12:00:00`);
    return Number.isNaN(date.getTime()) ? undefined : date;
  }

  function emitDates(dates: Date[]): void {
    if (mode === 'range') {
      const nextStart = toIso(dates[0]);
      const nextEnd = toIso(dates[1]);
      if (nextStart !== startDate || nextEnd !== endDate) {
        startDate = nextStart;
        endDate = nextEnd;
        dispatch('change', { startDate: nextStart, endDate: nextEnd });
      }
      return;
    }
    const next = toIso(dates[0]);
    if (next !== value) {
      value = next;
      dispatch('change', { value: next });
    }
  }

  onMount(() => {
    fp = flatpickr(inputEl, {
      dateFormat: 'Y-m-d',
      locale: flatpickrLocale(localeCode),
      defaultDate: mode === 'range'
        ? [startDate, endDate].filter(Boolean) as string[]
        : (value || undefined),
      minDate: fromIso(min),
      maxDate: fromIso(max),
      onChange: (dates: Date[]) => emitDates(dates),
    }) as Instance | null;
  });

  // 外部值变化（切周/切月等调用点重置）同步进 flatpickr
  $: if (fp && mode === 'range') {
    fp.setDate([startDate, endDate].filter(Boolean) as string[], false);
  }
  $: if (fp && mode !== 'range' && value) {
    fp.setDate(value, false);
  }

  onDestroy(() => {
    fp?.destroy();
  });
</script>

<input
  bind:this={inputEl}
  class="md-date-input {triggerClass}"
  type="text"
  readonly
  placeholder={mode === 'range' ? 'YYYY-MM-DD – YYYY-MM-DD' : 'YYYY-MM-DD'}
  aria-label={mode === 'range' ? 'YYYY-MM-DD – YYYY-MM-DD' : 'YYYY-MM-DD'}
/>

<style>
  /* MD outlined text field 观感的只读触发器 */
  .md-date-input {
    height: 2.5rem;
    padding: 0 0.875rem;
    border: 1px solid var(--md-theme-border-strong);
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--md-theme-on-surface);
    font-size: 0.875rem;
    font-family: inherit;
    cursor: pointer;
  }

  .md-date-input:focus {
    outline: none;
    border-color: var(--md-theme-primary);
    border-width: 2px;
    padding: 0 0.8125rem;
  }

  /* flatpickr 弹层 MD 皮肤 */
  :global(.flatpickr-calendar) {
    border-radius: var(--radius-md);
    box-shadow:
      0 5px 5px -3px rgba(0, 0, 0, 0.2),
      0 8px 10px 1px rgba(0, 0, 0, 0.14),
      0 3px 14px 2px rgba(0, 0, 0, 0.12);
    background: var(--md-theme-surface);
    color: var(--md-theme-on-surface);
    font-family: inherit;
  }

  :global(.flatpickr-calendar .flatpickr-months .flatpickr-month) {
    background: var(--md-theme-surface);
    color: var(--md-theme-on-surface);
  }

  :global(.flatpickr-calendar .flatpickr-weekdays) {
    background: var(--md-theme-surface);
  }

  :global(.flatpickr-calendar span.flatpickr-weekday) {
    background: var(--md-theme-surface);
    color: var(--md-theme-on-surface-medium);
  }

  :global(.flatpickr-calendar .flatpickr-day.selected),
  :global(.flatpickr-calendar .flatpickr-day.startRange),
  :global(.flatpickr-calendar .flatpickr-day.endRange) {
    background: var(--md-theme-primary);
    border-color: var(--md-theme-primary);
    color: var(--md-theme-on-primary);
  }

  :global(.flatpickr-calendar .flatpickr-day.inRange) {
    background: var(--md-theme-primary-container);
    border-color: var(--md-theme-primary-container);
    color: var(--md-theme-on-primary-container);
  }

  :global(.flatpickr-calendar .flatpickr-day:hover) {
    background: var(--md-theme-border-subtle);
    border-color: transparent;
  }

  :global(.flatpickr-calendar .flatpickr-day.today) {
    border-color: var(--md-theme-primary);
  }

  :global(.flatpickr-calendar .flatpickr-current-month .flatpickr-monthDropdown-months),
  :global(.flatpickr-calendar .numInputWrapper input) {
    color: var(--md-theme-on-surface);
  }

  :global(.flatpickr-calendar .flatpickr-prev-month svg),
  :global(.flatpickr-calendar .flatpickr-next-month svg) {
    fill: var(--md-theme-on-surface);
  }
</style>
