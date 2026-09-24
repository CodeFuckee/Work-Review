// 概览页 Plane 项目概览的纯展示逻辑：把已入库的当日 Plane 工作项
// （get_plane_work_items_for_date 返回、updated_at 倒序）按项目聚合为
// 「当日变动数 + 最近变动工作项」行模型。

export interface PlaneProjectOverviewRow {
  /** 项目名（plane_work_items.project_name 原样） */
  project: string;
  /** 该项目当日变动工作项条数 */
  count: number;
  /** 组内 updated_at 最大的工作项名称 */
  latestName: string;
  /** 上述工作项的状态名 */
  latestState: string;
  /** 上述工作项的更新时间（UTC unix 秒） */
  latestUpdatedAt: number;
}

export interface PlaneWorkItemOverviewInput {
  project_name: string;
  name: string;
  state_name: string;
  updated_at: number;
}

export function buildPlaneProjectOverview(
  items: readonly PlaneWorkItemOverviewInput[],
): PlaneProjectOverviewRow[] {
  const groups = new Map<string, PlaneProjectOverviewRow>();
  for (const item of items) {
    const existing = groups.get(item.project_name);
    if (!existing) {
      // 保持首次出现顺序：入库查询按 updated_at 倒序返回，
      // 首次出现即最近有变动的项目排在前（与 GitLab 卡的输入序约定一致）。
      groups.set(item.project_name, {
        project: item.project_name,
        count: 1,
        latestName: item.name,
        latestState: item.state_name,
        latestUpdatedAt: item.updated_at,
      });
      continue;
    }
    existing.count += 1;
    // latest 取组内 updated_at 最大的条目；并列时保留先出现者（倒序输入下即先入库者）
    if (item.updated_at > existing.latestUpdatedAt) {
      existing.latestName = item.name;
      existing.latestState = item.state_name;
      existing.latestUpdatedAt = item.updated_at;
    }
  }
  return [...groups.values()];
}
