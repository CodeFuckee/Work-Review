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

export type MergedTimelineRow<TActivity extends { timestamp: number }> =
  | { kind: 'activity'; key: number; activity: TActivity }
  | { kind: 'commit'; key: number; commit: GitlabCommitRow };

export function mergeTimelineRows<TActivity extends { timestamp: number }>(
  activities: readonly TActivity[],
  commits: readonly GitlabCommitRow[],
): MergedTimelineRow<TActivity>[] {
  const rows: MergedTimelineRow<TActivity>[] = [
    ...activities.map((activity) => ({ kind: 'activity' as const, key: activity.timestamp, activity })),
    ...commits.map((commit) => ({ kind: 'commit' as const, key: commit.committed_at, commit })),
  ];
  // key 相同时提交排在前（瞬时事件先于区间记录的展示习惯）；排序需稳定
  rows.sort((left, right) => left.key - right.key || (left.kind === 'commit' ? -1 : 0) - (right.kind === 'commit' ? -1 : 0));
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
