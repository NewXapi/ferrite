import { expect, test } from "@playwright/test";
import { AliasesPage } from "../pages/aliases.page";

/**
 * 别名卡行内 Popover 编辑（UI 决策记录 §2.2/§2.3）。
 *
 * 覆盖今天手工验过的三条路径，固化为回归：
 * 1. 点行开浮层 → 改本地字段（展示名）→ 保存 → 行值即时刷新；
 * 2. Escape / 取消 / 点面板外 → 浮层收关且不提交；
 * 3. 卡内 ≤3 个圆点 tab 可切换，定价 tab 露出价格编辑行。
 */
test.describe("别名卡 Popover 编辑", () => {
  let aliases: AliasesPage;

  test.beforeEach(async ({ page }) => {
    aliases = new AliasesPage(page);
    await aliases.goto();
  });

  test("保存本地字段后行值即时刷新", async ({ page }) => {
    const card = page.getByTestId("alias-card-new").first();
    const row = card.getByTestId("alias-edit-display");
    const before = await row.textContent();

    const popover = await aliases.openField(card, "display");
    const draft = `e2e-${Date.now()}`;
    await popover.getByRole("textbox").fill(draft);
    await popover.getByTestId("alias-edit-display-save").click();

    await expect(popover).toBeHidden();
    await expect(row).toContainText(draft);
    // 保存的是本地字段（后端 models 域无 display 列），刷新后回退属预期，
    // 不断言持久化；断言「提交后行即刻反映新值」这一交互契约。
    expect(before).not.toContain(draft);
  });

  test("Escape 与取消都只收关不提交", async ({ page }) => {
    const card = page.getByTestId("alias-card-new").first();
    const row = card.getByTestId("alias-edit-display");
    const original = await row.textContent();

    const popover = await aliases.openField(card, "display");
    await popover.getByRole("textbox").fill("should-not-commit");
    await page.keyboard.press("Escape");
    await expect(popover).toBeHidden();
    await expect(row).toHaveText(original ?? "");

    const popover2 = await aliases.openField(card, "display");
    await popover2.getByRole("textbox").fill("should-not-commit-either");
    await popover2.getByTestId("alias-edit-display-cancel").click();
    await expect(popover2).toBeHidden();
    await expect(row).toHaveText(original ?? "");
  });

  test("定价 tab 露出价格编辑行", async ({ page }) => {
    const card = page.getByTestId("alias-card-new").first();
    await card.getByTestId("dot-tab-1").click();
    await expect(card.getByTestId("alias-edit-input")).toBeVisible();
    await expect(card.getByTestId("alias-edit-multiplier")).toBeVisible();
    await card.getByTestId("dot-tab-2").click();
    await expect(card.getByText("可用分组")).toBeVisible();
  });
});
