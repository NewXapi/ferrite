import { expect, test } from "@playwright/test";
import { AdminPage } from "../pages/admin.page";

/** 与 ui::CARD_PAGE_SIZE 对齐（5 列网格 × 3 行）。 */
const PAGE_SIZE = 15;

/**
 * 卡片网格分页器（tab 式页码条）。
 *
 * 契约：固定页大小 15，超过一页才渲染页码条；页码条在区段标题旁；翻页只换
 * 可见卡片、不发新请求；不足一页时整条不渲染。
 *
 * 期望值（总条数 / 页数 / 每页张数）全部从导航时拿到的真实 2xx 响应推导，
 * 不写死种子数据——旧版写死「users 38 条」换环境即假失败。testid 为维护者
 * 确认的最终态：列表容器 <prefix>-list、卡片 <tab>-card（aliases/channels
 * 沿用 *-card-new）、页码按钮 <prefix>-pager-page-{i}（0 基，当前页带
 * aria-current="page"）。
 *
 * subscriptions 按约定并入：源里 subscriptions-list + subscriptions-pager
 * （卡片 subscription-card，接口 /api/subscriptions）本轮就位。
 */
const TABS = [
  { hash: "users", listTestId: "users-list", cardTestId: "user-card", pagerTestId: "users-pager", apiPath: "/api/user/users" },
  { hash: "aliases", listTestId: "aliases-list", cardTestId: "alias-card-new", pagerTestId: "aliases-pager", apiPath: "/api/models" },
  { hash: "channels", listTestId: "channels-list", cardTestId: "channel-card-new", pagerTestId: "channels-pager", apiPath: "/api/channel" },
  { hash: "groups", listTestId: "groups-list", cardTestId: "group-card", pagerTestId: "groups-pager", apiPath: "/api/group" },
  { hash: "redemptions", listTestId: "redemptions-list", cardTestId: "redemption-card", pagerTestId: "redemptions-pager", apiPath: "/api/redemption" },
  { hash: "subscriptions", listTestId: "subscriptions-list", cardTestId: "subscription-card", pagerTestId: "subscriptions-pager", apiPath: "/api/subscriptions" },
];

test.describe("卡片网格分页器", () => {
  for (const tab of TABS) {
    test(`${tab.hash}：分页条出现性与真实条数一致，翻页只换可见卡片`, async ({ page }) => {
      const admin = new AdminPage(page);
      const seen = admin.countResponses(tab.apiPath); // 观察先于导航
      const res = await admin.gotoTab(tab.hash, tab.listTestId, tab.apiPath);

      // UI 对拉到的 items 做前端切片，期望值与 UI 同源推导
      const total: number = (await res!.json()).items.length;
      const pages = Math.ceil(total / PAGE_SIZE);

      // 卡片查询圈在列表容器内：users/groups/redemptions 在网格外还渲染一张
      // 同组件原型卡（*-card-prototype 区），不圈定会多算
      const cards = page.getByTestId(tab.listTestId).getByTestId(tab.cardTestId);
      const pager = page.getByTestId(tab.pagerTestId);

      if (pages <= 1) {
        await expect(pager).toHaveCount(0); // 不足一页整条不渲染
        await expect(cards).toHaveCount(total);
        return;
      }

      await expect(pager).toBeVisible();
      await expect(page.getByTestId(`${tab.pagerTestId}-page-${pages - 1}`)).toBeVisible();
      await expect(page.getByTestId(`${tab.pagerTestId}-page-0`)).toHaveAttribute("aria-current", "page");
      await expect(cards).toHaveCount(Math.min(PAGE_SIZE, total));

      const firstBefore = await cards.first().getAttribute("aria-label");
      const before = seen.length;

      // 负向观察窗先于点击挂上：翻页若偷发请求，窗口内必现形
      const quiet = admin.requestArrivedWithin(tab.apiPath, 3_000);
      await page.getByTestId(`${tab.pagerTestId}-page-1`).click();

      // 消费者可见的翻页过渡：当前页高亮迁移 + 可见卡片换人 + 张数对齐
      await expect(page.getByTestId(`${tab.pagerTestId}-page-1`)).toHaveAttribute("aria-current", "page");
      await expect(cards).toHaveCount(Math.min(PAGE_SIZE, total - PAGE_SIZE));
      await expect(cards.first()).not.toHaveAttribute("aria-label", firstBefore ?? "");

      expect(await quiet).toBe(false); // 纯前端切片，不发新请求
      expect(seen.length).toBe(before);
    });
  }
});
