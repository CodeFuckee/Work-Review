// 仓库选择器的纯逻辑：搜索过滤、勾选切换、保存前规范化。
// 允许列表语义与配置层一致：忽略大小写、精确匹配；空选集 = 统计全部。

export function filterProjectOptions(projects: string[], query: string): string[] {
  const keyword = query.trim().toLowerCase();
  if (keyword === '') {
    return [...projects];
  }
  return projects.filter((path) => path.toLowerCase().includes(keyword));
}

export function toggleProject(selected: string[], path: string): string[] {
  const key = path.trim().toLowerCase();
  const without = selected.filter((item) => item.trim().toLowerCase() !== key);
  if (without.length !== selected.length) {
    return without;
  }
  return [...selected, path.trim()];
}

export function normalizeProjectSelection(selected: string[]): string[] {
  const seen = new Set<string>();
  const result: string[] = [];
  for (const item of selected) {
    const trimmed = item.trim();
    if (trimmed === '') continue;
    const key = trimmed.toLowerCase();
    if (seen.has(key)) continue;
    seen.add(key);
    result.push(trimmed);
  }
  return result;
}
