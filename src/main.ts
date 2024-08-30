//import { invoke } from "@tauri-apps/api/tauri";

let changelogEl: HTMLElement | null;

export function toggleChangelog() {
  if (changelogEl == null) {
    return;
  }

  changelogEl.hidden = !changelogEl.hidden;
}

declare global {
  interface Window { toggleChangelog: any }
}


window.toggleChangelog = toggleChangelog;


window.addEventListener("DOMContentLoaded", () => {
  changelogEl = document.getElementById("changelog");
});
