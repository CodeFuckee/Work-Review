import test from 'node:test';
import assert from 'node:assert';
import type { AssistantState } from './assistant.ts';

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

function snapshotFromStore(store: {
  subscribe(run: (state: AssistantState) => void): () => void;
}): AssistantState {
  let state: AssistantState | undefined;
  const unsubscribe = store.subscribe((value) => {
    state = value;
  });
  unsubscribe();
  assert.ok(state, '订阅应立即收到当前状态');
  return state;
}

test('重启恢复时应清除 sending 与 streaming 残留（幽灵生成气泡）', async () => {
  // 模拟应用在流式中途退出：持久化里留下 streaming:true / sending:true
  backing.set(
    'work-review-assistant-state',
    JSON.stringify({
      messages: [
        { id: 'm1', role: 'user', content: '今天做了什么' },
        { id: 'm2', role: 'assistant', content: '正在生成…', streaming: true },
      ],
      selectedModelId: '__basic__',
      hasUserSelectedModel: false,
      sending: true,
      sendingRequestId: 'req-1',
      conversationId: 7,
    }),
  );

  const { assistantStore } = await import('./assistant.ts');
  const state = snapshotFromStore(assistantStore);

  assert.equal(state.sending, false, 'sending 应强制复位');
  assert.equal(state.sendingRequestId, null, 'sendingRequestId 应强制复位');
  assert.equal(state.conversationId, 7, '会话 id 应原样保留');
  const streaming = state.messages.find((message) => message.id === 'm2');
  assert.ok(streaming, '恢复后应保留 m2 消息');
  assert.equal(streaming.streaming, false, '恢复的消息不得残留 streaming:true');
});
