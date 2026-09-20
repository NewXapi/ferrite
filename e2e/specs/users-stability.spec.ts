import { expect, test } from "@playwright/test";
import { AdminPage } from "../pages/admin.page";

/**
 * 用户列表请求稳定性（#230 修复的回归固化）。
 *
 * 修复前：取数 effect 用 users.read() 订阅自己写回的信号，每条响应都重跑
 * effect → 约 1 秒 25 次 GET 死循环 + 主线程钉死。修复后：列表稳定后不再
 * 自发重拉，手动刷新只 +1。
 *
 * 不断言「启动恰好 1 次」：debug-auto-login 首次 401 后会自动重登再拉一次，
 * 启动期 2 次属预期；要看门的是**稳定之后的无限增长**。就绪信号取「首个
 * 2xx 列表响应 + 首卡渲染」，不拍固定时长等启动期结束。
 */
const USERS_API = "/api/user/users";

test.describe("用户列表请求稳定性", () => {
  test("稳定后静置不重拉，手动刷新只多发一次", async ({ page }) => {
    const admin = new AdminPage(page);
    const seen = admin.countResponses(USERS_API); // 观察先于导航
    await admin.gotoTab("users", "users-panel", USERS_API);
    await expect(page.getByTestId("users-list").getByTestId("user-card").first()).toBeVisible();

    // 首个 2xx 已到达（就绪），此时取稳定基线
    const baseline = seen.length;
    expect(baseline).toBeGreaterThanOrEqual(1);

    expect(await admin.requestArrivedWithin(USERS_API, 3_000)).toBe(false); // 修复前这里会持续增长

    await page.getByTestId("refresh-users").click();
    await expect.poll(() => seen.length, { timeout: 10_000 }).toBe(baseline + 1);
    expect(await admin.requestArrivedWithin(USERS_API, 3_000)).toBe(false);
  });

  test("搜索是纯本地过滤，不发新请求", async ({ page }) => {
    const admin = new AdminPage(page);
    const seen = admin.countResponses(USERS_API); // 观察先于导航
    const res = await admin.gotoTab("users", "users-panel", USERS_API);

    // 过滤词与期望命中数都从真实响应推导，不写死种子用户名
    const items: Array<{ username: string; email: string }> = (await res!.json()).items;
    const query = items[0].username;
    const needle = query.toLowerCase();
    const matched = items.filter(
      (u) => u.username.toLowerCase().includes(needle) || u.email.toLowerCase().includes(needle),
    ).length;
    expect(matched).toBeGreaterThan(0); // 首项自身必命中

    const cards = page.getByTestId("users-list").getByTestId("user-card");
    const before = seen.length;

    // 负向观察窗先于输入挂上：本地过滤若误发请求，窗口内必现形
    const quiet = admin.requestArrivedWithin(USERS_API, 3_000);
    await page.getByTestId("users-search").fill(query);

    // 消费者可见：只剩命中项（分页上限 15），且每张可见卡都含过滤词
    await expect(cards).toHaveCount(Math.min(15, matched));
    for (const card of await cards.all()) {
      expect((await card.textContent())?.toLowerCase()).toContain(needle);
    }
    expect(await quiet).toBe(false);
    expect(seen.length).toBe(before);
  });
});
