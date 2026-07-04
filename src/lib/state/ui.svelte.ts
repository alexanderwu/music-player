// Tiny UI state: which view is showing (no router needed — PLAN 3.2),
// plus cross-component signals like "focus the search box".

export const ui = $state({
  view: "library" as "library" | "playlist" | "settings",
  /** Incremented to ask the library view to focus its search input. */
  searchFocusTick: 0,
  /** True while files are being dragged over the window. */
  dropActive: false,
});

export function showLibrary() {
  ui.view = "library";
}

export function focusSearch() {
  ui.view = "library";
  ui.searchFocusTick += 1;
}
