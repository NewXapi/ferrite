// 给 leptos 页面(origin 8081)签发 browser grant，写到 leptos 自己的 site 目录。
// 不能复用主仓 aino-bridge：它的 connection.json 输出路径相对脚本位置写死，
// 两个桥会互相覆盖同一个文件。
import { readFile, writeFile } from 'node:fs/promises';
import { homedir } from 'node:os';

const SERVICE_USER_FILE = `${homedir()}/.ainotation/service/connection.json`;
const OUT = '/home/hathaway/projects/ferrite/.wt/leptos-web/target/site/assets/ainotation/connection.json';
const PROJECT_NAME = 'ferrite-admin';
const ORIGIN = 'http://127.0.0.1:8081';
const RENEW_MS = 2 * 60 * 1000;

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
  // service 重启会换端口，url 与管理 token 都运行时读发现文件。
  const svc = JSON.parse(await readFile(SERVICE_USER_FILE, 'utf8'));
  const { url, token } = svc;

  const projects = await api(url, token, '/control/projects');
  const project = (projects.projects ?? []).find((p) => (p.name ?? p.config?.name) === PROJECT_NAME);
  if (!project) throw new Error('project ferrite-admin not registered; run just aino-service once');
  const projectId = project.projectId ?? project.config?.projectId;

  let grantToken;
  async function issue() {
    const grant = await api(url, token, '/control/grants', 'POST', { kind: 'browser', projectId, origin: ORIGIN });
    grantToken = grant.token;
    await writeFile(OUT, `${JSON.stringify({ url, token: grant.token }, null, 2)}\n`);
    console.log('grant issued', grant.grantId, 'expires', new Date(grant.expiresAt).toISOString());
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
