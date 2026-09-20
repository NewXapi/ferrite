import { defineConfig, devices } from "@playwright/test";

/**
 * Ferrite 管理页浏览器 E2E。
 *
 * 定位：补 Rust wire 契约测试（tests/）覆盖不到的**浏览器层**回归——
 * wasm 渲染、行内 Popover 编辑、分页切片、请求稳定性。后端用真实 dev 后端
 * （`just dev-backend start` 起的共享 3211），不做 API mock：本仓库的 UI
 * 缺陷（忙轮询白屏、effect 自循环）恰恰只在真实请求回路里复现。
 *
 * 本地跑法（必须 debug 档）：
 *   just dev-web 8095 debug          # debug-auto-login 免登录（admin_dev）
 *   cd e2e && bunx playwright test   # E2E_BASE_URL 默认 http://127.0.0.1:8095
 *
 * 普通档（`just dev-web 8095`）没有自动登录：页面停在登录/注册页，全部用例
 * 假失败——旧文档漏写 `debug`，踩过这个坑。debug 档首拉 wasm 是 CPU-heavy
 * 构建（首次数百秒），按仓库约定重命令套 `cpulimit -l 65 -i --`
 * （见 .agent/rules/testing-ci.md）。
 *
 * 本套件不在当前 CI 里（详见 e2e/README.md）：没有 browser job，chromium 用
 * 本机 Playwright 缓存里已有的构建（executablePath 指向 ~/.cache/ms-playwright），
 * 不为跑 smoke 额外下载；`E2E_CHROMIUM` 可覆盖路径。
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
