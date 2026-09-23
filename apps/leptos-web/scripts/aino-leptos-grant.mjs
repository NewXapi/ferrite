// 给 leptos 页面(origin 8081)签发 browser grant，写到 leptos 自己的 site 目录。
// 不能复用主仓 aino-bridge：它的 connection.json 输出路径相对脚本位置写死，
// 两个桥会互相覆盖同一个文件。
import { readFile, writeFile } from 'node:fs/promises';
import { homedir } from 'node:os';

const SERVICE_USER_FILE = `${homedir()}/.ainotation/service/connection.json`;
const OUT = '/home/hathaway/projects/ferrite/.wt/leptos-web/target/site/assets/ainotation/connection.json';
const ADMIN_IIFE_SRC = '/home/hathaway/projects/ferrite/apps/admin-web/assets/ainotation/ainotation.iife.js';
const TARGET_IIFE = '/home/hathaway/projects/ferrite/.wt/leptos-web/target/site/assets/ainotation/ainotation.iife.js';
const PROJECT_NAME = 'ferrite-admin';
const ORIGIN = 'http://127.0.0.1:8081';
const RENEW_MS = 2 * 60 * 1000;

async function ensureLeptosBundle() {
  try {
    let content = await readFile(ADMIN_IIFE_SRC, 'utf8');
    // Patch discovery order to same-origin first (leptos grant) before bridge. Prevents 403 on /sessions sync.
    // Matches the exact string baked into the admin-web bundle.
    content = content.replace(
      `["http://127.0.0.1:44090/connection.json","/assets/ainotation/connection.json"]`,
      `["/assets/ainotation/connection.json","http://127.0.0.1:44090/connection.json"]`
    );
    await writeFile(TARGET_IIFE, content);
    console.log('✓ leptos ainotation.iife.js copied from admin-web and patched (same-origin first)');
  } catch (e) {
    console.warn('bundle patch skipped:', e.message);
  }
}

async function api(url, token, path, method = 'GET', body) {
  const res = await fetch(url + path, {
    method,
    headers: { Authorization: `Bearer ${token}`, ...(body ? { 'Content-Type': 'application/json' } : {}) },
    body: body ? JSON.stringify(body) : undefined,
  });
  const text = await res.text();
  if (!res.ok) throw new Error(`${method} ${path} -> ${res.status} ${text.slice(0, 200)}`);
  return text ? JSON.parse(text) : null;
}

async function main() {
  await ensureLeptosBundle();

  // service 重启会换端口，url 与管理 token 都运行时读发现文件。
  const svc = JSON.parse(await readFile(SERVICE_USER_FILE, 'utf8'));
  const { url, token } = svc;

  const projects = await api(url, token, '/control/projects');
  const project = (projects.projects ?? []).find((p) => (p.name ?? p.config?.name) === PROJECT_NAME);
  if (!project) throw new Error('project ferrite-admin not registered; run just aino-service once');
  const projectId = project.projectId ?? project.config?.projectId;

  let grantToken;
  // 必须把 grant 返回出去：调用方靠它拿到 grantId 去 renew。
  // 之前漏了 return，导致 `grant` 恒为 undefined，下一轮 renew 抛
  // "Cannot read properties of undefined (reading 'grantId')"，
  // 于是每轮都走 catch 重新签发新 token —— 浏览器手里的旧 token 仍会过期，
  // 表现为页面开满一个 RENEW_MS 周期后批注 403。
  async function issue() {
    const grant = await api(url, token, '/control/grants', 'POST', { kind: 'browser', projectId, origin: ORIGIN });
    grantToken = grant.token;
    await writeFile(OUT, `${JSON.stringify({ url, token: grant.token }, null, 2)}\n`);
    console.log('grant issued', grant.grantId, 'expires', new Date(grant.expiresAt).toISOString());
    return grant;
  }

  let grant = await issue();
  setInterval(async () => {
    try {
      grant = await api(url, token, `/control/grants/${grant.grantId}/renew`, 'POST');
      await writeFile(OUT, `${JSON.stringify({ url, token: grantToken }, null, 2)}\n`);
      console.log('renewed, expires', new Date(grant.expiresAt).toISOString());
    } catch (error) {
      console.error('renew failed, re-issuing:', String(error).slice(0, 160));
      grant = await issue();
    }
  }, RENEW_MS);
  console.log('leptos connection file at', OUT);
}

main().catch((e) => { console.error('fatal:', e); process.exit(1); });