import { test, expect } from "@playwright/test";
import type { Snapshot } from "../../src/types";
const state: Snapshot = {
  folders: [],
  rules: [],
  history: [],
  busy: false,
  safe_mode: false,
  settings: {
    language: "fr",
    theme: "dark",
    automatic: false,
    close_to_tray: true,
    notifications: true,
    stabilization_seconds: 5,
    reconciliation_seconds: 300,
    scan_on_start: false,
    onboarding_done: true,
    reduced_motion: false,
    history_days: 0,
    update_checks: false,
  },
};
for (const width of [390, 720, 1180]) {
  test(`isolated IPC fixture: empty UI, folder/rule form, themes and language at ${width}px`, async ({
    page,
  }) => {
    await page.setViewportSize({ width, height: 900 });
    const errors: string[] = [];
    page.on("pageerror", (e) => errors.push(e.message));
    await page.route("**/src/main.tsx*", async (route) => {
      const response = await route.fetch();
      const source = await response.text();
      await route.fulfill({
        response,
        body: `import { mockIPC } from '/node_modules/@tauri-apps/api/mocks.js';
const fixture = ${JSON.stringify(state)};
window.isTauri = true;
mockIPC((command,args) => {
  if (command === 'snapshot') return structuredClone(fixture);
  if (command === 'app_version') return '0.1.0';
  if (command === 'plugin:autostart|is_enabled') return false;
  if (command === 'plugin:dialog|open') return 'C:\\\\isolated-fixture\\\\input';
  if (command === 'save_folder') { fixture.folders.push({...args.folder,id:'fixture-folder'}); return fixture.folders.at(-1); }
  if (command === 'save_rule') { fixture.rules.push({...args.rule,id:'fixture-rule'}); return fixture.rules.at(-1); }
  if (command === 'save_settings') { fixture.settings=args.settings; return; }
  return null;
},{shouldMockEvents:true});
${source}`,
      });
    });
    await page.goto("/");
    await expect(page.getByText("Votre espace, en ordre.")).toBeVisible();
    const contrast = await page
      .getByRole("button", { name: "Organiser maintenant", exact: true })
      .last()
      .evaluate((element) => {
        const style = getComputedStyle(element);
        const luminance = (color: string) =>
          color
            .match(/\d+/g)!
            .slice(0, 3)
            .map((n) => Number(n) / 255)
            .map((n) =>
              n <= 0.04045 ? n / 12.92 : ((n + 0.055) / 1.055) ** 2.4,
            )
            .reduce((sum, n, i) => sum + n * [0.2126, 0.7152, 0.0722][i], 0);
        const foreground = luminance(style.color),
          background = luminance(style.backgroundColor);
        return (
          (Math.max(foreground, background) + 0.05) /
          (Math.min(foreground, background) + 0.05)
        );
      });
    expect(contrast).toBeGreaterThanOrEqual(4.5);
    await page.screenshot({
      path: `work/screenshots/overview-${width}.png`,
      fullPage: true,
    });
    await page
      .getByRole("button", { name: "Dossiers surveillés", exact: true })
      .click();
    await page
      .getByRole("button", { name: "Ajouter un dossier", exact: true })
      .click();
    await page.getByLabel("Nom", { exact: true }).fill("Isolated fixture");
    await page
      .getByRole("button", { name: "Choisir un dossier", exact: true })
      .click();
    await page
      .getByRole("button", { name: "Enregistrer", exact: true })
      .click();
    await expect(
      page.getByRole("heading", { name: "Isolated fixture" }),
    ).toBeVisible();
    await page.getByRole("button", { name: "Règles", exact: true }).click();
    await page
      .getByRole("button", { name: "Créer une règle", exact: true })
      .click();
    await page
      .getByRole("combobox", { name: "Modèle", exact: true })
      .selectOption("documents");
    await page.getByLabel("Isolated fixture", { exact: true }).check();
    await page
      .getByLabel("Destination", { exact: true })
      .fill("C:\\isolated-fixture\\output");
    await page.getByRole("switch", { name: "Activé", exact: true }).check();
    await page.screenshot({
      path: `work/screenshots/rule-editor-${width}.png`,
      fullPage: true,
    });
    await page
      .getByRole("button", { name: "Enregistrer", exact: true })
      .click();
    await expect(
      page.getByRole("heading", { name: "Documents", exact: true }),
    ).toBeVisible();
    await expect(
      page.getByRole("button", { name: "Créer une règle", exact: true }),
    ).toBeFocused();
    await page.getByRole("button", { name: "Paramètres", exact: true }).click();
    await page
      .getByRole("combobox", { name: "Apparence", exact: true })
      .selectOption("light");
    await expect(page.locator("html")).toHaveAttribute("data-theme", "light");
    await page
      .getByRole("combobox", { name: "Langue", exact: true })
      .selectOption("en");
    await expect(
      page.getByRole("heading", { name: "Settings", exact: true }),
    ).toBeVisible();
    expect(
      await page.evaluate(
        () => document.documentElement.scrollWidth <= innerWidth,
      ),
    ).toBe(true);
    expect(errors).toEqual([]);
  });
}
