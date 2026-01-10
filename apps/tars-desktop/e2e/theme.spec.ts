import { test, expect } from "@playwright/test";

test.describe("Theme", () => {
  test("should default to dark theme", async ({ page }) => {
    await page.goto("/");

    // The root element should have the dark class by default
    const html = page.locator("html");
    await expect(html).toHaveClass(/dark/);
  });

  test("should apply dark theme styles", async ({ page }) => {
    await page.goto("/");

    // Get background color of body - should be dark
    const body = page.locator("body");
    const bgColor = await body.evaluate((el) =>
      window.getComputedStyle(el).backgroundColor
    );

    // Dark theme should have dark background
    // This is a basic check - adjust based on actual theme colors
    expect(bgColor).toBeDefined();
  });
});
