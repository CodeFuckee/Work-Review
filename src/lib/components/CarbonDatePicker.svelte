<script lang="ts">
  // Carbon DatePicker 薄封装：复刻 LocalizedDatePicker 的对外 API
  // （mode/value/startDate/endDate/localeCode/min/max/triggerClass + change 事件），
  // 使 Overview/Timeline/Report 的调用点只需更换 import 即可迁移。
  // 日期一律走 ISO（YYYY-MM-DD）字符串，与原组件契约一致。
  import { createEventDispatcher } from 'svelte';
  import DatePicker from 'carbon-components-svelte/src/DatePicker/DatePicker.svelte';

  // 宽化为 string：调用点传入字面量时由内部收窄
  export let mode: 'single' | 'range' | string = 'single';
  export let localeCode = 'zh-CN';
  export let value = '';
  export let startDate = '';
  export let endDate = '';
  export let min = '';
  export let max = '';
  // API 兼容位：旧组件的触发器样式类（Carbon 自带输入样式，不再需要，仅吞掉该属性）
  export let triggerClass = '';

  const dispatch = createEventDispatcher<{ change: { value?: string; startDate?: string; endDate?: string } }>();

  // flatpickr locale 键映射（carbon-components-svelte 内部传给 flatpickr）
  function flatpickrLocale(code: string): string {
    switch (code) {
      case 'zh-CN': return 'zh';
      case 'zh-TW': return 'zh_tw';
      case 'ar': return 'ar';
      default: return 'en';
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

  function applyRange(dates: Date[]): void {
    const nextStart = toIso(dates[0]);
    const nextEnd = toIso(dates[1]);
    if (nextStart !== startDate || nextEnd !== endDate) {
      startDate = nextStart;
      endDate = nextEnd;
      dispatch('change', { startDate: nextStart, endDate: nextEnd });
    }
  }

  $: datePickerType = mode === 'range' ? 'range' : 'simple';
  $: carbonLocale = flatpickrLocale(localeCode);
  $: valueFrom = mode === 'range' ? startDate : undefined;
  $: valueTo = mode === 'range' ? endDate : undefined;
</script>

<DatePicker
  datePickerType={datePickerType as 'simple' | 'range'}
  value={mode === 'single' ? value : ''}
  {valueFrom}
  {valueTo}
  dateFormat="Y-m-d"
  locale={carbonLocale as never}
  minDate={fromIso(min)}
  maxDate={fromIso(max)}
  placeholder={mode === 'range' ? 'YYYY-MM-DD – YYYY-MM-DD' : 'YYYY-MM-DD'}
  on:change={(e) => {
    // simple 模式 detail 为格式化字符串；range 模式为 { selectedDates }
    if (typeof e.detail === 'string') {
      if (e.detail !== value) {
        value = e.detail;
        dispatch('change', { value: e.detail });
      }
      return;
    }
    applyRange(e.detail.selectedDates as Date[]);
  }}
/>
