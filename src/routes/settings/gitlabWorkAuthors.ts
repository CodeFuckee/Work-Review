// 「纳入工作的作者」输入解析：把设置页多行/逗号分隔的作者名单
// 解析为去空白、去空、忽略大小写去重（保留首次写法）的数组。

export function parseWorkAuthorsInput(text: string): string[] {
  const seen = new Set<string>();
  const result: string[] = [];
  for (const raw of text.split(/[\n,，]/)) {
    const item = raw.trim();
    if (item === '') continue;
    const key = item.toLowerCase();
    if (seen.has(key)) continue;
    seen.add(key);
    result.push(item);
  }
  return result;
}
