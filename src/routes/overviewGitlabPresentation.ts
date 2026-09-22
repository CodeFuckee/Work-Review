// 概览页 GitLab 提交统计的纯展示逻辑：项目行构建与合计。
// SQL 侧已按提交次数倒序、项目路径升序排序，这里只做展示转换与汇总。

export interface GitlabProjectStat {
  project: string;
  commits: number;
  additions: number;
  deletions: number;
}

export interface GitlabProjectRow {
  /** 完整项目路径（tooltip 展示） */
  project: string;
  /** 路径末段（列表主文案，如 group/alpha -> alpha） */
  shortName: string;
  commits: number;
  additions: number;
  deletions: number;
  /** 进度条宽度百分比（以提交数最多的项目为满格，最低 2% 可见） */
  barPercent: number;
}

export interface GitlabProjectTotals {
  commits: number;
  additions: number;
  deletions: number;
}

/** 取项目路径末段；无分隔符或以分隔符结尾时取剥尾后的完整路径 */
export function gitlabProjectShortName(project: string): string {
  const trimmed = project.trim().replace(/\/+$/, '');
  const separatorIndex = trimmed.lastIndexOf('/');
  if (separatorIndex < 0) return trimmed;
  return trimmed.slice(separatorIndex + 1);
}

export function buildGitlabProjectRows(
  stats: readonly GitlabProjectStat[],
): GitlabProjectRow[] {
  const rows: GitlabProjectRow[] = stats.map((stat) => ({
    project: stat.project,
    shortName: gitlabProjectShortName(stat.project),
    commits: stat.commits,
    additions: stat.additions,
    deletions: stat.deletions,
    barPercent: 0,
  }));
  // 进度条以提交数最多的项目为满格（与时间线/应用使用的柱宽口径一致），最低 2% 可见
  const maxCommits = Math.max(0, ...rows.map((row) => row.commits));
  for (const row of rows) {
    row.barPercent = maxCommits > 0 ? Math.max((row.commits / maxCommits) * 100, 2) : 0;
  }
  return rows;
}

export function sumGitlabProjectRows(
  rows: readonly GitlabProjectRow[],
): GitlabProjectTotals {
  return rows.reduce<GitlabProjectTotals>(
    (totals, row) => ({
      commits: totals.commits + row.commits,
      additions: totals.additions + row.additions,
      deletions: totals.deletions + row.deletions,
    }),
    { commits: 0, additions: 0, deletions: 0 },
  );
}
