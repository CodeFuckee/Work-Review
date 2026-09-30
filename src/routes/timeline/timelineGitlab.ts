// 时间线页的多源合并逻辑：把已入库的应用活动记录、GitLab 提交、文件监控
// 事件与 Plane 工作项按时间戳交错排序为统一行模型（commit 行以项目路径充当
// 应用名，planeItem 行以项目名充当应用名）。

export interface GitlabCommitRow {
  sha: string;
  date: string;
  committed_at: number;
  project: string;
  branch: string;
  short_id: string;
  title: string;
  additions: number | null;
  deletions: number | null;
  author_name: string;
  author_email: string;
}

export interface FileEventRow {
  path: string;
  event_type: string;
  size: number | null;
  occurred_at: number;
  date: string;
}

export interface PlaneWorkItemRow {
  plane_item_id: string;
  project_name: string;
  name: string;
  state_name: string;
  updated_at: number;
  url: string | null;
}

export type MergedTimelineRow<TActivity extends { timestamp: number }> =
  | { kind: 'activity'; key: number; activity: TActivity }
  | { kind: 'commit'; key: number; commit: GitlabCommitRow }
  | { kind: 'fileEvent'; key: number; event: FileEventRow }
  | { kind: 'planeItem'; key: number; item: PlaneWorkItemRow };

export function mergeTimelineRows<TActivity extends { timestamp: number }>(
  activities: readonly TActivity[],
  commits: readonly GitlabCommitRow[],
  fileEvents: readonly FileEventRow[] = [],
  planeItems: readonly PlaneWorkItemRow[] = [],
): MergedTimelineRow<TActivity>[] {
  const rows: MergedTimelineRow<TActivity>[] = [
    ...activities.map((activity) => ({ kind: 'activity' as const, key: activity.timestamp, activity })),
    ...commits.map((commit) => ({ kind: 'commit' as const, key: commit.committed_at, commit })),
    ...fileEvents.map((event) => ({ kind: 'fileEvent' as const, key: event.occurred_at, event })),
    ...planeItems.map((item) => ({ kind: 'planeItem' as const, key: item.updated_at, item })),
  ];
  // 时间线页为「最新在最上」（prepareTimelineActivities 的既有约定），合并后
  // 必须延续降序；同 key 时提交行在前、活动行次之、文件事件行再次、
  // Plane 工作项最后（外部系统条目按信号强度依次让位于本地活动记录）。
  const kindOrder = (kind: MergedTimelineRow<TActivity>['kind']): number =>
    kind === 'commit' ? -1 : kind === 'fileEvent' ? 1 : kind === 'planeItem' ? 2 : 0;
  rows.sort(
    (left, right) => right.key - left.key || kindOrder(left.kind) - kindOrder(right.kind),
  );
  return rows;
}

// 提交统计后缀："+12/-3"；无统计（部分仓库不返回）时空串。
export function formatCommitStatsSuffix(
  additions: number | null,
  deletions: number | null,
): string {
  if (additions === null || deletions === null) return '';
  return ` (+${additions}/-${deletions})`;
}

// ---- 提交详情抽屉「修改文件」区块：diff 解析与状态判定 ----

export interface GitlabDiffFileLike {
  new_file: boolean;
  renamed_file: boolean;
  deleted_file: boolean;
}

export type DiffLineKind = 'add' | 'del' | 'hunk' | 'ctx' | 'meta';

export interface DiffLine {
  kind: DiffLineKind;
  text: string;
}

export function diffFileState(file: GitlabDiffFileLike): 'added' | 'deleted' | 'renamed' | 'modified' {
  if (file.new_file) return 'added';
  if (file.deleted_file) return 'deleted';
  if (file.renamed_file) return 'renamed';
  return 'modified';
}

// unified diff 逐行归类：+/− 增删（`+++`/`---` 文件头除外）、@@ hunk 头、
// `\ No newline` 提示与文件头行归 meta，其余上下文；纯空白行（含 diff 末尾
// 换行产生的空行）也归 meta 保持行数对齐。maxLines 截断防超长 diff
// （minified 文件等）撑爆 webview，截断数随结果返回。
export function parseDiffLines(
  diff: string,
  maxLines: number,
): { lines: DiffLine[]; truncated: number } {
  if (diff.trim() === '') {
    return { lines: [], truncated: 0 };
  }
  const raw = diff.split('\n');
  const lines: DiffLine[] = [];
  for (const text of raw) {
    let kind: DiffLineKind;
    if (text.startsWith('+++') || text.startsWith('---') || text.startsWith('\\')) {
      kind = 'meta';
    } else if (text.startsWith('@@')) {
      kind = 'hunk';
    } else if (text.startsWith('+')) {
      kind = 'add';
    } else if (text.startsWith('-')) {
      kind = 'del';
    } else if (text.trim() === '') {
      // 纯空白/空行：diff 末尾通常带一个换行产生的空行，跳过；
      // 上下文空行（首空格）保留为 ctx
      kind = text.startsWith(' ') ? 'ctx' : 'meta';
    } else {
      kind = 'ctx';
    }
    if (lines.length >= maxLines) {
      const truncated = raw.length - lines.length;
      return { lines, truncated: Math.max(truncated, 0) };
    }
    lines.push({ kind, text });
  }
  return { lines, truncated: 0 };
}
