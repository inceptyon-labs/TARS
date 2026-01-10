import { test, expect } from "@playwright/test";

test.describe("App", () => {
  test("should load without JavaScript errors", async ({ page }) => {
    const errors: string[] = [];

    page.on("pageerror", (error) => {
      errors.push(error.message);
    });

    await page.goto("/");

    // Wait for app to fully load
    await page.waitForLoadState("networkidle");

    // Check no JavaScript errors occurred
    expect(errors).toHaveLength(0);
  });

  test("should display the main layout", async ({ page }) => {
    await page.goto("/");

    // Main content area should be visible
    await expect(page.locator("main, [role='main']").first()).toBeVisible();
  });

  test("should have working navigation sidebar", async ({ page }) => {
    await page.goto("/");

    // Navigation should have multiple links
    const navLinks = page.locator("nav a, aside a");
    const count = await navLinks.count();

    expect(count).toBeGreaterThan(0);
  });

  test("should display TARS branding", async ({ page }) => {
    await page.goto("/");

    // Look for TARS text or logo
    const branding = page.getByText("TARS");
    await expect(branding.first()).toBeVisible();
  });

  test("should be responsive", async ({ page }) => {
    // Test at mobile viewport
    await page.setViewportSize({ width: 375, height: 667 });
    await page.goto("/");

    // App should still be usable
    await expect(page.locator("body")).toBeVisible();

    // Test at tablet viewport
    await page.setViewportSize({ width: 768, height: 1024 });
    await expect(page.locator("body")).toBeVisible();

    // Test at desktop viewport
    await page.setViewportSize({ width: 1920, height: 1080 });
    await expect(page.locator("body")).toBeVisible();
  });
});
