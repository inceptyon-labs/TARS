import { test, expect } from "@playwright/test";

test.describe("Navigation", () => {
  test("should redirect from root to projects page", async ({ page }) => {
    await page.goto("/");

    await expect(page).toHaveURL("/projects");
  });

  test("should navigate to profiles page", async ({ page }) => {
    await page.goto("/");

    await page.click('text=Profiles');

    await expect(page).toHaveURL("/profiles");
  });

  test("should navigate to skills page", async ({ page }) => {
    await page.goto("/");

    await page.click('text=Skills');

    await expect(page).toHaveURL("/skills");
  });

  test("should navigate to agents page", async ({ page }) => {
    await page.goto("/");

    await page.click('text=Agents');

    await expect(page).toHaveURL("/agents");
  });

  test("should navigate to commands page", async ({ page }) => {
    await page.goto("/");

    await page.click('text=Commands');

    await expect(page).toHaveURL("/commands");
  });

  test("should navigate to hooks page", async ({ page }) => {
    await page.goto("/");

    await page.click('text=Hooks');

    await expect(page).toHaveURL("/hooks");
  });

  test("should navigate to mcp page", async ({ page }) => {
    await page.goto("/");

    await page.click('text=MCP');

    await expect(page).toHaveURL("/mcp");
  });

  test("should navigate to plugins page", async ({ page }) => {
    await page.goto("/");

    await page.click('text=Plugins');

    await expect(page).toHaveURL("/plugins");
  });

  test("should navigate to updates page", async ({ page }) => {
    await page.goto("/");

    await page.click('text=Updates');

    await expect(page).toHaveURL("/updates");
  });
});
