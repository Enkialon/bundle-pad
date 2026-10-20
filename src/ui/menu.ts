export interface MenuItem {
  label: string;
  shortcut?: string;
  run: () => void;
}

export interface MenuGroup {
  label: string;
  items: MenuItem[];
}

export function mountMenu(root: HTMLElement, groups: MenuGroup[]): void {
  root.replaceChildren();
  let openPanel: HTMLElement | null = null;

  const close = () => {
    openPanel?.classList.remove("open");
    openPanel = null;
  };

  for (const group of groups) {
    const menu = document.createElement("div");
    menu.className = "menu";
    const button = document.createElement("button");
    button.type = "button";
    button.className = "menu-button";
    button.textContent = group.label;
    const panel = document.createElement("div");
    panel.className = "menu-panel";
    panel.hidden = false;

    button.addEventListener("click", (event) => {
      event.stopPropagation();
      const willOpen = openPanel !== panel;
      close();
      if (willOpen) {
        panel.classList.add("open");
        openPanel = panel;
      }
    });

    for (const item of group.items) {
      const entry = document.createElement("button");
      entry.type = "button";
      entry.className = "menu-item";
      const label = document.createElement("span");
      label.textContent = item.label;
      entry.append(label);
      if (item.shortcut) {
        const shortcut = document.createElement("kbd");
        shortcut.textContent = item.shortcut;
        entry.append(shortcut);
      }
      entry.addEventListener("click", () => {
        close();
        item.run();
      });
      panel.append(entry);
    }

    menu.append(button, panel);
    root.append(menu);
  }

  document.addEventListener("click", close);
  document.addEventListener("keydown", (event) => {
    if (event.key === "Escape") close();
  });
}
