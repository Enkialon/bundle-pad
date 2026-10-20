export interface ShortcutActions {
  newDocument: () => void;
  openMarkdown: () => void;
  save: () => void;
  saveAs: () => void;
}

export function bindShortcuts(actions: ShortcutActions): void {
  window.addEventListener("keydown", (event) => {
    if (!event.ctrlKey || event.altKey || event.metaKey) return;
    const key = event.key.toLowerCase();
    if (key === "s" && event.shiftKey) {
      event.preventDefault();
      actions.saveAs();
      return;
    }
    if (key === "s") {
      event.preventDefault();
      actions.save();
      return;
    }
    if (key === "o" && !event.shiftKey) {
      event.preventDefault();
      actions.openMarkdown();
      return;
    }
    if (key === "n") {
      event.preventDefault();
      actions.newDocument();
    }
  });
}
