import { expect, test } from "@playwright/test";
for (const width of [390, 720, 1180]) {
  test(`browser shell reports missing desktop engine at ${width}px`, async ({
    page,
  }) => {
    await page.setViewportSize({ width, height: 800 });
    const errors: string[] = [];
    page.on("pageerror", (e) => errors.push(e.message));
    await page.goto("/");
    await expect(page.getByRole("alert")).toContainText(
      "Ouvrez l’application Windows",
    );
    await expect(
      page.getByRole("button", { name: "Règles", exact: true }),
    ).toBeVisible();
    expect(
      await page.evaluate(
        () => document.documentElement.scrollWidth <= innerWidth,
      ),
    ).toBe(true);
    expect(errors).toEqual([]);
  });
}
