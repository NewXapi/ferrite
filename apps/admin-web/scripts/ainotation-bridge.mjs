// Ainotation MCP 同步桥（dev 工具，仅本地）。
//
// 背景：ainotation 的浏览器→service 同步需要同源中间件桥（上游仅 Vite 插件内置）。
// dx serve 挂不了中间件，本脚本用 service 的 control API 以 node 侧身份补齐两件事：
//   1. 注册项目（ferrite-admin）→ 拿 projectId；
//   2. 给页面 origin 签发 browser grant（5 分钟租约）并循环续租，grant token 写入
//      apps/admin-web/assets/ainotation/connection.json（gitignore），SDK 挂载时读取。
//
// 用法：先起本地服务 `npx --yes @ainotation/mcp@beta service`（或由 omp 的 MCP connect 自动拉起），
// 然后 `node scripts/ainotation-bridge.mjs`（保持前台运行即持续续租）。
import { readFile, writeFile } from 'node:fs/promises';
import { createServer } from 'node:http';

const SERVICE_USER_FILE = '/home/hathaway/.ainotation/service/connection.json';
const SDK_CONNECTION = new URL('../assets/ainotation/connection.json', import.meta.url);
const ORIGIN = 'http://127.0.0.1:8092';
const FILE_PORT = 44090;
const PROJECT_NAME = 'ferrite-admin';
const PROJECT_DIRECTORY = '/home/hathaway/projects/ferrite';
const RENEW_INTERVAL_MS = 2 * 60 * 1000;

async function api(url, token, path, method = 'GET', body) {
  const res = await fetch(url + path, {
    method,
    headers: {
      Authorization: `Bearer ${token}`,
      ...(body ? { 'Content-Type': 'application/json' } : {}),
    },
    body: body ? JSON.stringify(body) : undefined,
  });
  const text = await res.text();
  if (!res.ok) throw new Error(`${method} ${path} -> ${res.status} ${text.slice(0, 200)}`);
  return text ? JSON.parse(text) : null;
}

async function main() {
  // service 可能由常驻 connect 进程稍后拉起，轮询等它的发现文件
  let svc;
  for (let i = 0; ; i++) {
    try {
      svc = JSON.parse(await readFile(SERVICE_USER_FILE, 'utf8'));
      break;
    } catch {
      if (i > 60) throw new Error('service connection.json not available after 120s');
      await new Promise((r) => setTimeout(r, 2000));
    }
  }
  const { url, token } = svc;

  // 1. 注册项目（幂等：已注册返回既有记录）
  let project;
  try {
    project = await api(url, token, '/control/projects', 'POST', {
      directory: PROJECT_DIRECTORY,
      declaration: { name: PROJECT_NAME },
    });
  } catch (error) {
    const projects = await api(url, token, '/control/projects');
    project = (projects.projects ?? []).find(
      (p) => (p.projectId ?? p.config?.projectId) && (p.name ?? p.config?.name) === PROJECT_NAME,
    );
    if (!project) throw error;
  }
  const projectId = project.projectId ?? project.config.projectId;
  console.log('project ready:', projectId, project.name ?? project.config?.name);

  // 2. 签发 grant 并写入 SDK 连接文件
  async function issue() {
    const grant = await api(url, token, '/control/grants', 'POST', {
      kind: 'browser',
      projectId,
      origin: ORIGIN,
    });
    await import('node:fs/promises').then((fs) =>
      fs.writeFile(SDK_CONNECTION, `${JSON.stringify({ url, token: grant.token }, null, 2)}\n`),
    );
    console.log('grant issued', grant.grantId, 'expires', new Date(grant.expiresAt).toISOString());
    return grant;
  }
  let grant = await issue();

  // 3. 续租循环（renew 保持同一 token 有效，无需重写连接文件）
  setInterval(async () => {
    try {
      grant = await api(url, token, `/control/grants/${grant.grantId}/renew`, 'POST');
      console.log('renewed, expires', new Date(grant.expiresAt).toISOString());
    } catch (error) {
      console.error('renew failed, re-issuing:', String(error).slice(0, 160));
      grant = await issue();
    }
  }, RENEW_INTERVAL_MS);

  // 4. 固定端口 CORS 端点：页面 SDK 挂载时取 {url, token}（dx 不服务 asset_dir 原始路径）
  createServer((req, res) => {
    res.setHeader('Access-Control-Allow-Origin', '*');
    res.setHeader('Content-Type', 'application/json');
    res.end(JSON.stringify({ url, token: grant.token }));
  }).listen(FILE_PORT, '127.0.0.1', () => console.log(`connection file served on :${FILE_PORT}`));
}

main().catch((error) => {
  console.error('bridge fatal:', error);
  process.exit(1);
});
