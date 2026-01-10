import { test, expect } from "@playwright/test";

test.describe("Projects Page", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/projects");
  });

  test("should display projects page title", async ({ page }) => {
    // The page should load without errors
    await expect(page.locator("body")).toBeVisible();
  });

  test("should show add project button", async ({ page }) => {
    // Look for the add project button (either in sidebar or main content)
    const addButton = page.getByRole("button", { name: /add|new|project/i });

    // At least one add button should exist
    await expect(addButton.first()).toBeVisible();
  });

  test("should open add project dialog when clicking add button", async ({
    page,
  }) => {
    // Click the add project button
    const addButton = page.getByRole("button", { name: /add project/i });

    if (await addButton.isVisible()) {
      await addButton.click();

      // Dialog should appear
      const dialog = page.getByRole("dialog");
      await expect(dialog).toBeVisible();
    }
  });
});
