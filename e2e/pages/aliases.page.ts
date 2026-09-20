import { expect, type Locator, type Page } from "@playwright/test";
import { AdminPage } from "./admin.page";

/** 字段 key → 浮层 aria-label（= 行左侧标签名）。 */
const POPOVER_LABELS: Record<string, string> = {
  name: "别名",
  display: "展示名",
  input: "输入",
  output: "输出",
  multiplier: "倍率",
};

/**
 * 别名 tab 页面对象：新卡牌（ui::AliasCard）的行内 Popover 编辑。
 *
 * 对应 UI 决策记录 §2.2：单字段点击行 → Popover → 保存；危险操作 → Dialog。
 * 只断言**不写后端**的路径（本地字段保存、Esc/取消收关、Dialog 开消）——
 * 别名 rename 走真实 PUT，留给需要写库的集成场景，避免污染 dev 种子数据。
 */
export class AliasesPage {
  readonly admin: AdminPage;
  readonly page: Page;

  constructor(page: Page) {
    this.page = page;
    this.admin = new AdminPage(page);
  }

  async goto() {
    await this.admin.gotoTab("aliases", "aliases-list");
    await expect(this.page.getByTestId("alias-card-new").first()).toBeVisible();
  }

  /** 打开某张卡某个字段的编辑浮层，返回浮层 locator。 */
  async openField(card: Locator, field: keyof typeof POPOVER_LABELS): Promise<Locator> {
    await card.getByTestId(`alias-edit-${field}`).click();
    const popover = card.getByRole("dialog", { name: POPOVER_LABELS[field] });
    await expect(popover).toBeVisible();
    return popover;
  }
}
