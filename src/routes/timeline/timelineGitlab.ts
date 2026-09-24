// 时间线页的 GitLab 提交合并逻辑：把已入库的提交行与应用活动记录
// 按时间戳交错排序为统一行模型（commit 行以项目路径充当应用名）。

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

export type MergedTimelineRow<TActivity extends { timestamp: number }> =
  | { kind: 'activity'; key: number; activity: TActivity }
  | { kind: 'commit'; key: number; commit: GitlabCommitRow }
  | { kind: 'fileEvent'; key: number; event: FileEventRow };

export function mergeTimelineRows<TActivity extends { timestamp: number }>(
  activities: readonly TActivity[],
  commits: readonly GitlabCommitRow[],
  fileEvents: readonly FileEventRow[] = [],
): MergedTimelineRow<TActivity>[] {
  const rows: MergedTimelineRow<TActivity>[] = [
    ...activities.map((activity) => ({ kind: 'activity' as const, key: activity.timestamp, activity })),
    ...commits.map((commit) => ({ kind: 'commit' as const, key: commit.committed_at, commit })),
    ...fileEvents.map((event) => ({ kind: 'fileEvent' as const, key: event.occurred_at, event })),
  ];
  // 时间线页为「最新在最上」（prepareTimelineActivities 的既有约定），合并后
  // 必须延续降序；同 key 时提交行在前、活动行次之、文件事件行最后。
  const kindOrder = (kind: MergedTimelineRow<TActivity>['kind']): number =>
    kind === 'commit' ? -1 : kind === 'fileEvent' ? 1 : 0;
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
