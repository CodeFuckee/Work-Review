// 时间线行的 Plane 工作项归因 join：时间线上的应用活动行、GitLab 提交行、
// 文件事件行共用同一套「时间戳落入归因会话窗口 [start, end] 即标注该工作项」
// 的判定（归因条目由后端按日现算，线性扫描即可，条目很少）。

export interface AttributionWindowLike {
  session_start: number;
  session_end: number;
}

export function attributionForTimestamp<T extends AttributionWindowLike>(
  attributions: readonly T[],
  timestamp: number,
): T | null {
  for (const attr of attributions) {
    if (timestamp >= attr.session_start && timestamp <= attr.session_end) {
      return attr;
    }
  }
  return null;
}

/**
 * 把归因结果附加到行数据上（行 + attribution 字段），供时间线模板直接读取。
 *
 * 归因 join 必须在这类响应式派生层完成而不是模板行内调函数：Svelte 只追踪
 * 表达式里词法出现的组件变量，行内 `fn(row.x)` 读不到归因列表的更新
 * （归因按日现算、晚于行渲染到达，行内计算会永远拿到空列表不刷新）。
 */
export function attachAttribution<TRow, TAttr extends AttributionWindowLike>(
  rows: readonly TRow[],
  attributions: readonly TAttr[],
  getTimestamp: (row: TRow) => number,
): Array<TRow & { attribution: TAttr | null }> {
  return rows.map((row) => ({
    ...row,
    attribution: attributionForTimestamp(attributions, getTimestamp(row)),
  }));
}
