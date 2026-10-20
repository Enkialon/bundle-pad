export interface MenuItem {
  label: string;
  shortcut?: string;
  separatorBefore?: boolean;
  run: () => void;
}

export interface MenuGroup {
  label: string;
  items: MenuItem[];
}

export function mountMenu(root: HTMLElement, groups: MenuGroup[]): void {
  root.replaceChildren();
  root.setAttribute("role", "menubar");
  let openMenu: HTMLElement | null = null;

  const close = () => {
    if (!openMenu) return;
    openMenu.classList.remove("open");
    const button = openMenu.querySelector(".menu-button");
    button?.classList.remove("open");
    button?.setAttribute("aria-expanded", "false");
    openMenu = null;
  };

  const open = (menu: HTMLElement) => {
    if (openMenu === menu) return;
    close();
    menu.classList.add("open");
    const button = menu.querySelector(".menu-button");
    button?.classList.add("open");
    button?.setAttribute("aria-expanded", "true");
    openMenu = menu;
  };

  for (const group of groups) {
    const menu = document.createElement("div");
    menu.className = "menu";
    const button = document.createElement("button");
    button.type = "button";
    button.className = "menu-button";
    button.textContent = group.label;
    button.setAttribute("role", "menuitem");
    button.setAttribute("aria-haspopup", "menu");
    button.setAttribute("aria-expanded", "false");
    const panel = document.createElement("div");
    panel.className = "menu-panel";
    panel.setAttribute("role", "menu");

    button.addEventListener("click", (event) => {
      event.stopPropagation();
      if (openMenu === menu) close();
      else open(menu);
    });
    button.addEventListener("mouseenter", () => {
      if (openMenu && openMenu !== menu) open(menu);
    });

    for (const item of group.items) {
      if (item.separatorBefore) {
        const separator = document.createElement("div");
        separator.className = "menu-separator";
        separator.setAttribute("role", "separator");
        panel.append(separator);
      }
      const entry = document.createElement("button");
      entry.type = "button";
      entry.className = "menu-item";
      entry.setAttribute("role", "menuitem");
      const label = document.createElement("span");
      label.textContent = item.label;
      entry.append(label);
      if (item.shortcut) {
        const shortcut = document.createElement("kbd");
        shortcut.textContent = item.shortcut;
        entry.append(shortcut);
      }
      entry.addEventListener("click", (event) => {
        event.stopPropagation();
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
