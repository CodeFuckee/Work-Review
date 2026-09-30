import test from 'node:test';
import assert from 'node:assert';
import type { AssistantMessageInput } from './assistant.ts';

// 伪造 window.localStorage：assistant.ts 模块加载时（createAssistantStore →
// loadState）即执行恢复，因此必须在动态导入之前就位。
const backing = new Map<string, string>();
(globalThis as unknown as { window: unknown }).window = {
  localStorage: {
    getItem: (key: string) => backing.get(key) ?? null,
    setItem: (key: string, value: string) => {
      backing.set(key, value);
    },
    removeItem: (key: string) => {
      backing.delete(key);
    },
  },
};

const STORAGE_KEY = 'work-review-assistant-state';

function snapshotFromStore(store: {
  subscribe(run: (state: { messages: unknown[] }) => void): () => void;
}): { messages: unknown[] } {
  let state: { messages: unknown[] } | undefined;
  const unsubscribe = store.subscribe((value) => {
    state = value;
  });
  unsubscribe();
  assert.ok(state, '订阅应立即收到当前状态');
  return state;
}

test('长会话追加消息不应在内存里截断旧消息', async () => {
  const { assistantStore } = await import('./assistant.ts');

  // 模拟 SQLite 长会话全量装入（超过持久化上限 40 条）
  const history: AssistantMessageInput[] = Array.from({ length: 50 }, (_, index) => ({
    id: `m${index}`,
    role: index % 2 === 0 ? 'user' : 'assistant',
    content: `历史消息 ${index}`,
  }));
  assistantStore.setConversation(1, history);

  assistantStore.appendMessage({ role: 'user', content: '新问题' });
  const state = snapshotFromStore(assistantStore);
  assert.equal(
    state.messages.length,
    51,
    'appendMessage 不得裁掉会话内已有消息（旧实现 slice(-40) 会截断）',
  );
  const first = state.messages[0] as { id?: string };
  assert.equal(first.id, 'm0', '最早的消息应仍然在内存态中');
});

test('持久化兜底应把 localStorage 裁到最近 40 条', async () => {
  const { assistantStore } = await import('./assistant.ts');

  const history: AssistantMessageInput[] = Array.from({ length: 50 }, (_, index) => ({
    id: `n${index}`,
    role: index % 2 === 0 ? 'user' : 'assistant',
    content: `历史消息 ${index}`,
  }));
  assistantStore.setConversation(2, history);
  assistantStore.appendMessage({ role: 'user', content: '触发持久化' });

  // persistState 由 500ms 防抖定时器触发
  await new Promise((resolve) => setTimeout(resolve, 700));

  const raw = backing.get(STORAGE_KEY);
  assert.ok(raw, '防抖后应已写入 localStorage');
  const persisted = JSON.parse(raw) as { messages: { id: string }[] };
  assert.ok(
    persisted.messages.length <= 40,
    `持久化消息数应 ≤40，实际 ${persisted.messages.length}`,
  );
  assert.equal(
    persisted.messages.some((message) => message.id === 'n0'),
    false,
    '最旧的超出上限的消息应被裁掉',
  );
  assert.equal(
    persisted.messages.some((message) => message.id === 'm0'),
    false,
    '裁剪后不应残留第一个测试用例的消息',
  );
});
