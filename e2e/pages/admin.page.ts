import { expect, type Page } from "@playwright/test";

/**
 * 管理台公共页面对象：导航到某个 tab，等面板容器 + 认证就绪的列表响应。
 *
 * 选择器约定（与仓库 .agent/skills/ui-validation 一致）：
 * - 交互元素用 `data-testid`，不用 class（样式调整即失效）；
 * - 断言用 role + name 的结构化快照思路，不用截图肉眼判断。
 *
 * 就绪判定不拍睡眠：传 `apiPath` 时在 `goto` **之前**挂上「首个 2xx 响应」
 * 监听（debug-auto-login 的首拉 401 → 自动重登 → 2xx 全程不漏检），面板
 * 容器可见且该响应到达才算就绪。401/5xx 不兜底成就绪——免登录链路真断了
 * 就超时失败，不用睡眠把认证失败蒙混过去。
 */
export class AdminPage {
  readonly page: Page;

  constructor(page: Page) {
    this.page = page;
  }

  /**
   * 打开管理台并跳到指定 hash 路由。
   *
   * @param apiPath 列表接口路径（pathname 精确匹配）。传入时返回该路径首个
   *   2xx 响应，调用方从中推导期望值（总条数等）；省略时只等面板容器，
   *   返回 null。
   */
  async gotoTab(hash: string, panelTestId: string, apiPath?: string) {
    // 观察先于导航：启动期请求（含自动登录后的首拉）一个不漏
    const ready = apiPath
      ? this.page.waitForResponse(
          (r) =>
            new URL(r.url()).pathname === apiPath && r.status() >= 200 && r.status() < 300,
          { timeout: 30_000 },
        )
      : null;
    ready?.catch(() => {}); // 面板断言先失败时，避免未处理拒绝

    await this.page.goto(`/#${hash}`);
    await expect(this.page.getByTestId(panelTestId)).toBeVisible({ timeout: 30_000 });
    return ready;
  }

  /** 左侧「管理」导航里的 tab 按钮（role=button + 可访问名）。 */
  tab(name: string) {
    return this.page.getByRole("button", { name, exact: true });
  }

  /** 计数指定路径的响应次数（请求稳定性断言用）。必须在导航/交互之前调用。 */
  countResponses(path: string) {
    const seen: number[] = [];
    this.page.on("response", (r) => {
      if (new URL(r.url()).pathname === path) seen.push(r.status());
    });
    return seen;
  }

  /**
   * 负向观察窗：`windowMs` 内出现该路径的新响应则返回 true。
   * 用于「翻页 / 静置 / 搜索不发请求」这类契约——窗口先于交互挂上，点击
   * 瞬间偷发的请求不会漏检；调用方对返回值断言 false。这是有界的测量窗，
   * 不是拍固定时长等渲染。
   */
  async requestArrivedWithin(path: string, windowMs: number): Promise<boolean> {
    let arrived = false;
    const onResponse = (r: { url: () => string }) => {
      if (new URL(r.url()).pathname === path) arrived = true;
    };
    this.page.on("response", onResponse);
    const { promise, resolve } = Promise.withResolvers<void>();
    setTimeout(resolve, windowMs);
    await promise;
    this.page.off("response", onResponse);
    return arrived;
  }
}
