import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import "./i18n";
import i18n from "./i18n";
import { App } from "./App";
import { RuleEditor } from "./editors";
import { useNavigation } from "./lib";
import type { Snapshot } from "./types";
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
const invoke = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/core", () => ({ isTauri: () => true, invoke }));
vi.mock("@tauri-apps/api/event", () => ({
  listen: () => Promise.resolve(() => {}),
}));
vi.mock("@tauri-apps/plugin-autostart", () => ({
  isEnabled: () => Promise.resolve(false),
}));
function mount() {
  invoke.mockImplementation((command: string) =>
    Promise.resolve(command === "app_version" ? "0.1.0" : state),
  );
  return render(
    <QueryClientProvider
      client={
        new QueryClient({ defaultOptions: { queries: { retry: false } } })
      }
    >
      <App />
    </QueryClientProvider>,
  );
}
afterEach(() => {
  cleanup();
  vi.clearAllMocks();
  useNavigation.setState({ page: "dashboard" });
});
describe("desktop interface", () => {
  it("renders real empty dashboard counters", async () => {
    mount();
    expect(
      await screen.findByText("Votre espace, en ordre."),
    ).toBeInTheDocument();
    expect(
      screen.getByText("Aucune opération pour le moment"),
    ).toBeInTheDocument();
    expect(screen.getAllByText("0")).toHaveLength(3);
  });
  it("navigates to folder management", async () => {
    mount();
    await screen.findByText("Votre espace, en ordre.");
    await userEvent.click(
      screen.getByRole("button", { name: "Dossiers surveillés" }),
    );
    expect(
      screen.getByText("Un premier dossier, pour commencer."),
    ).toBeInTheDocument();
  });
  it("refuses invalid rule form before IPC", async () => {
    render(<RuleEditor folders={[]} close={() => {}} saved={() => {}} />);
    await userEvent.type(screen.getByLabelText("Destination"), "C:\\Test");
    await userEvent.click(screen.getByRole("button", { name: "Enregistrer" }));
    expect(await screen.findAllByText("Ce champ est obligatoire")).toHaveLength(
      2,
    );
    expect(invoke).not.toHaveBeenCalled();
  });
  it("shows actionable backend failure", async () => {
    mount();
    await screen.findByText("Votre espace, en ordre.");
    invoke.mockRejectedValueOnce("Permission denied");
    await userEvent.click(screen.getByRole("button", { name: "Dépôt GitHub" }));
    expect(await screen.findByText("Permission denied")).toBeInTheDocument();
  });
  it("has matching French and English translation keys", () => {
    expect(
      Object.keys(i18n.getResourceBundle("fr", "translation")).sort(),
    ).toEqual(Object.keys(i18n.getResourceBundle("en", "translation")).sort());
  });
});
