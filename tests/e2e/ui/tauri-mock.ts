import type { Page } from '@playwright/test';

// 模拟 Tauri IPC：@tauri-apps/api v2 的 invoke 走 window.__TAURI_INTERNALS__.invoke。
// addInitScript 的参数必须可结构化克隆，因此这里只传"命令 → 返回值"的纯数据表；
// 未在表中的命令一律 resolve(null)（与页面里的可选链/兜底逻辑配合）。
export type MockSpec = Record<string, unknown>;

export async function installTauriMock(page: Page, spec: MockSpec): Promise<void> {
  await page.addInitScript((mockSpec: MockSpec) => {
    const w = window as unknown as {
      __TAURI_INTERNALS__: {
        metadata: unknown;
        plugins: Record<string, unknown>;
        transformCallback: (cb: unknown) => unknown;
        invoke: (cmd: string, args?: unknown, options?: unknown) => Promise<unknown>;
      };
    };
    w.__TAURI_INTERNALS__ = {
      metadata: {
        currentWindow: { label: 'main' },
        currentWebview: { label: 'main', windowLabel: 'main' },
      },
      plugins: {},
      transformCallback: (cb) => cb,
      invoke: (cmd: string) => {
        if (Object.prototype.hasOwnProperty.call(mockSpec, cmd)) {
          return Promise.resolve(mockSpec[cmd]);
        }
        return Promise.resolve(null);
      },
    };
  }, spec);
}

/** 出厂默认文本模型：后端 get_config 归一化后为 builtin 供应商 + AI 增强模式。 */
export function factoryDefaultTextModel(): MockSpec {
  return {
    get_config: {
      ai_mode: 'summary',
      text_model: {
        provider: 'builtin',
        endpoint: '',
        api_key: null,
        model: '',
      },
    },
  };
}

const BUILTIN_PROVIDER_ENTRY = {
  id: 'builtin',
  name: '内置模型',
  description: '软件内置的兜底大模型，无需任何配置',
  default_endpoint: '',
  default_model: '',
  requires_api_key: false,
  supports_vision: false,
  builtin: true,
};

const OLLAMA_PROVIDER_ENTRY = {
  id: 'ollama',
  name: 'Ollama (本地)',
  description: '在本机运行的开源大模型，数据不出本机',
  default_endpoint: 'http://localhost:11434',
  default_model: 'qwen3',
  requires_api_key: false,
  supports_vision: false,
};

/** 无内置模型时的供应商列表（等价于本地开发未注入 key 的构建） */
export const OLLAMA_PROVIDER_ENTRY_ONLY = [OLLAMA_PROVIDER_ENTRY];

/** 出厂默认（无内置）：provider 保持 ollama 出厂形状、日报为基础模板 */
export function factoryDefaultTextModelOllama(): MockSpec {
  return {
    get_config: {
      ai_mode: 'local',
      text_model: {
        provider: 'ollama',
        endpoint: 'http://localhost:11434',
        api_key: null,
        model: 'qwen3',
      },
    },
  };
}

/** 用户已配置自定义模型：非本地端点、带 key，且日报处于 AI 增强模式（渲染模型分区）。 */
export function customTextModel(): MockSpec {
  return {
    get_config: {
      ai_mode: 'summary',
      text_model: {
        provider: 'openai',
        endpoint: 'https://api.openai.com/v1',
        api_key: 'sk-mock-key',
        model: 'gpt-4o',
      },
    },
  };
}

/** 各页面启动/渲染所需命令的通用兜底值，测试按需覆写。 */
export function baseSpec(overrides: MockSpec = {}): MockSpec {
  return {
    get_config: {},
    get_storage_stats: {},
    get_data_dir: 'C:\\mock\\data',
    get_default_data_dir: 'C:\\mock\\data',
    get_runtime_platform: 'windows',
    get_categories: [],
    get_update_settings: { autoCheck: true, lastCheckTime: 0, checkIntervalHours: 24 },
    has_builtin_model: true,
    get_ai_providers: [BUILTIN_PROVIDER_ENTRY, OLLAMA_PROVIDER_ENTRY],
    ...overrides,
  };
}
