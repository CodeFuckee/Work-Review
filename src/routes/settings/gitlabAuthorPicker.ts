// 作者选择器的纯逻辑：搜索过滤、勾选切换、与既有名单合并。
// 名单语义与后端一致：只写邮箱（提交 author_email 忽略大小写精确匹配）；
// 合并为追加语义——手输条目保留，勾选邮箱去重后追加到末尾。

export interface GitlabAuthorOption {
  name: string;
  email: string;
  commits: number;
  is_current_user: boolean;
}

export function filterAuthorOptions(
  authors: GitlabAuthorOption[],
  query: string,
): GitlabAuthorOption[] {
  const keyword = query.trim().toLowerCase();
  if (keyword === '') {
    return [...authors];
  }
  return authors.filter(
    (author) =>
      author.name.toLowerCase().includes(keyword) ||
      author.email.toLowerCase().includes(keyword),
  );
}

export function toggleAuthor(selectedEmails: string[], email: string): string[] {
  const key = email.trim().toLowerCase();
  const without = selectedEmails.filter((item) => item.trim().toLowerCase() !== key);
  if (without.length !== selectedEmails.length) {
    return without;
  }
  return [...selectedEmails, email.trim()];
}

// 保存时与既有「纳入工作的作者」名单合并：追加语义——已有条目（含手输
// 姓名）原样保留在前，勾选邮箱里与既有条目忽略大小写重复的不再追加。
export function mergeWorkAuthorsWithPicked(
  existingAuthors: string[],
  pickedEmails: string[],
): string[] {
  const seen = new Set(
    existingAuthors.map((item) => item.trim().toLowerCase()).filter((key) => key !== ''),
  );
  const appended: string[] = [];
  for (const email of pickedEmails) {
    const trimmed = email.trim();
    if (trimmed === '') continue;
    const key = trimmed.toLowerCase();
    if (seen.has(key)) continue;
    seen.add(key);
    appended.push(trimmed);
  }
  return [...existingAuthors, ...appended];
}

// 已写入名单的邮箱在候选列表里预勾选（忽略大小写匹配）。
export function preselectedAuthorEmails(
  authors: GitlabAuthorOption[],
  existingAuthors: string[],
): string[] {
  const existing = new Set(
    existingAuthors.map((item) => item.trim().toLowerCase()).filter((key) => key !== ''),
  );
  return authors
    .filter((author) => existing.has(author.email.trim().toLowerCase()))
    .map((author) => author.email);
}
