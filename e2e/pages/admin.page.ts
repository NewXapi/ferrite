import { expect, type Page } from "@playwright/test";

/**
 * 管理台公共页面对象：导航到某个 tab 并等它的面板容器就绪。
 *
 * 选择器约定（与仓库 .agent/skills/ui-validation 一致）：
 * - 交互元素用 `data-testid`，不用 class（样式调整即失效）；
 * - 断言用 role + name 的结构化快照思路，不用截图肉眼判断。
 */
export class AdminPage {
  readonly page: Page;

  constructor(page: Page) {
    this.page = page;
  }

  /** 打开管理台并跳到指定 hash 路由，等面板容器出现。 */
  async gotoTab(hash: string, panelTestId: string) {
    await this.page.goto(`/#${hash}`);
    await expect(this.page.getByTestId(panelTestId)).toBeVisible({ timeout: 30_000 });
  }

  /** 左侧「管理」导航里的 tab 按钮（role=button + 可访问名）。 */
  tab(name: string) {
    return this.page.getByRole("button", { name, exact: true });
  }

  /** 计数指定路径的响应次数（请求稳定性断言用）。 */
  countResponses(path: string) {
    const seen: number[] = [];
    this.page.on("response", (r) => {
      if (new URL(r.url()).pathname === path) seen.push(r.status());
    });
    return seen;
  }
}
