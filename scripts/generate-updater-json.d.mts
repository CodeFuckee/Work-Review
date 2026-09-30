// generate-updater-json.mjs 的类型声明（供 *.test.ts 以 TypeScript 导入）
export interface UpdaterManifest {
  version: string;
  notes: string;
  pub_date: string;
  platforms: Record<string, { url: string; signature: string }>;
}

export interface BuildUpdaterManifestOptions {
  version: string;
  urlBase: string;
  /** Array<[平台名, 产物本地路径]> */
  platforms: Array<[string, string]>;
  /** release notes 内容（非文件路径） */
  notes: string;
}

export function parseArgs(argv: string[]): {
  version?: string;
  urlBase?: string;
  out?: string;
  notesFile?: string;
  platform: Array<[string, string]>;
};

export function buildUpdaterManifest(options: BuildUpdaterManifestOptions): UpdaterManifest;

export function assertPlatformVersionConsistency(
  manifest: UpdaterManifest,
  expectedVersion: string,
): void;
