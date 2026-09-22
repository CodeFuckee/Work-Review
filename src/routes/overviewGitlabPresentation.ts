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
  return stats.map((stat) => ({
    project: stat.project,
    shortName: gitlabProjectShortName(stat.project),
    commits: stat.commits,
    additions: stat.additions,
    deletions: stat.deletions,
  }));
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
