import { defineConfig, devices } from "@playwright/test";

/**
 * Ferrite 管理页浏览器 E2E。
 *
 * 定位：补 Rust wire 契约测试（tests/）覆盖不到的**浏览器层**回归——
 * wasm 渲染、行内 Popover 编辑、请求稳定性。后端用真实 dev 后端
 * （`just dev-web <port>` 起的共享 3211 + debug-auto-login 免登录），
 * 不做 API mock：本仓库的 UI 缺陷（忙轮询白屏、effect 自循环）恰恰
 * 只在真实请求回路里复现。
 *
 * 本地跑法：
 *   just dev-web 8095                 # 或任何已起的预览
 *   cd e2e && bunx playwright test    # E2E_BASE_URL 默认 http://127.0.0.1:8095
 *
 * chromium 用 Playwright 缓存里已有的构建（executablePath 指向
 * ~/.cache/ms-playwright），不为 CI 之外的环境额外下载；CI 上改用
 * `npx playwright install --with-deps chromium` 时把 executablePath 删掉即可
 * （见 README）。
 */
export default defineConfig({
  testDir: "./specs",
  fullyParallel: false,
  workers: 1,
  retries: process.env.CI ? 1 : 0,
  reporter: process.env.CI ? [["line"], ["html", { open: "never" }]] : [["list"]],
  timeout: 45_000,
  expect: { timeout: 10_000 },
  use: {
    baseURL: process.env.E2E_BASE_URL ?? "http://127.0.0.1:8095",
    trace: "on-first-retry",
    screenshot: "only-on-failure",
    video: "off",
    launchOptions: {
      // 复用本机 Playwright 缓存里的 chromium（版本与 @playwright/test 解耦，
      // 避免为跑一条 smoke 再下一遍 150MB 浏览器）。
      executablePath:
        process.env.E2E_CHROMIUM ??
        `${process.env.HOME}/.cache/ms-playwright/chromium-1243/chrome-linux64/chrome`,
      args: ["--no-sandbox", "--disable-dev-shm-usage"],
    },
  },
  projects: [{ name: "chromium", use: { ...devices["Desktop Chrome"] } }],
});
