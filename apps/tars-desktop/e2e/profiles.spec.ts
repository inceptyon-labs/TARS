import { test, expect } from "@playwright/test";

test.describe("Profiles Page", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/profiles");
  });

  test("should display profiles page", async ({ page }) => {
    await expect(page.locator("body")).toBeVisible();
  });

  test("should show create profile button or empty state", async ({ page }) => {
    // Either show profiles list or empty state with create button
    const createButton = page.getByRole("button", {
      name: /create|new|profile/i,
    });
    const emptyState = page.getByText(/no profiles/i);

    // One of these should be visible
    const hasCreateButton = await createButton.first().isVisible();
    const hasEmptyState = await emptyState.isVisible();

    expect(hasCreateButton || hasEmptyState).toBeTruthy();
  });
});
