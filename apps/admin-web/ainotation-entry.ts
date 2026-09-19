// Ainotation 开发标注工具入口（仅开发环境，见 main.rs 的 debug_assertions 门控）。
// 打包：bun run aino → assets/ainotation/ainotation.iife.js
//
// MCP 接线：运行时 fetch 同目录的 connection.json（gitignore，由 `service` 的
// ~/.ainotation/service/connection.json 拷贝生成，见 AINOTATION.md）；缺失时降级为
// 纯本地模式（标注/复制/导出可用，不与 Agent 同步）。
import { createAinotation } from '@ainotation/sdk';

const PROJECT_ID = 'ferrite-admin';

function mount(options) {
  const inspector = createAinotation({ projectId: PROJECT_ID, ...options });
  void inspector.mount();
}

function start(mcp) {
  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', () => mount(mcp), { once: true });
  } else {
    mount(mcp);
  }
}

fetch('/assets/ainotation/connection.json', { cache: 'no-store' })
  .then((r) => (r.ok ? r.json() : null))
  .then((cfg) => start(cfg ? { mcp: { endpoint: cfg.url, token: cfg.token } } : {}))
  .catch(() => start({}));
