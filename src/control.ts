import { invoke } from "@tauri-apps/api/tauri";

let pageMsgEl: HTMLElement | null;
let mainPanelEl: HTMLElement | null;
let shockStrengthEl: HTMLInputElement | null;
let vibrateStrengthEl: HTMLInputElement | null;
let shockBtn: HTMLButtonElement | null;
let vibrateBtn: HTMLButtonElement | null;
let beepBtn: HTMLButtonElement | null;
let qtshockIp: string;



function checkIpAddress(ip: string) { 
    const ipv4Pattern =  
        /^(\d{1,3}\.){3}\d{1,3}$/; 
    const ipv6Pattern =  
        /^([0-9a-fA-F]{1,4}:){7}[0-9a-fA-F]{1,4}$/; 
    return ipv4Pattern.test(ip) || ipv6Pattern.test(ip); 
} 

async function shock() {
  if (shockStrengthEl) {
    console.log(await invoke("shock", {shocker: 0, strength: shockStrengthEl.value }));
  }
}

async function vibrate() {
    if (vibrateStrengthEl) {
      console.log(await invoke("vibrate", {shocker: 0, strength: vibrateStrengthEl.value }));
    }
}

async function beep() {
    console.log(await invoke("beep", {shocker: 0}));
}

window.addEventListener("DOMContentLoaded", async () => {
    pageMsgEl = document.getElementById("page-msg");
    if (!pageMsgEl) return;
    mainPanelEl = document.getElementById("main-panel");
    if (!mainPanelEl) return;
    mainPanelEl.style.display = "none";

    qtshockIp = await invoke("load_local_ip");
    if (!checkIpAddress(qtshockIp)) {
        mainPanelEl.style.display = "none";
        pageMsgEl.innerHTML = qtshockIp;
        return;
    }

    mainPanelEl.style.display = "flex";
    pageMsgEl.innerHTML = "";
    shockStrengthEl = document.getElementById("shock-strength") as HTMLInputElement;
    shockStrengthEl.addEventListener("change", (e) => {
        if (e.target) {
            let strength = parseInt((e.target as HTMLInputElement).value);
            if (strength < 1 || strength > 99) {
                if (!shockStrengthEl) return;
                shockStrengthEl.value = "24";
            }
        }
    });
    vibrateStrengthEl = document.getElementById("vibrate-strength") as HTMLInputElement;
    vibrateStrengthEl.addEventListener("change", (e) => {
        if (e.target) {
            let strength = parseInt((e.target as HTMLInputElement).value);
            if (strength < 1 || strength > 99) {
                if (!vibrateStrengthEl) return;
                vibrateStrengthEl.value = "24";
            }
        }
    });

    shockBtn = document.getElementById("shock-btn") as HTMLButtonElement;
    vibrateBtn = document.getElementById("vibrate-btn") as HTMLButtonElement;
    beepBtn = document.getElementById("beep-btn") as HTMLButtonElement;
    shockBtn.addEventListener("click", () => {
        shock();
    });
    vibrateBtn.addEventListener("click", () => {
        vibrate();
    });
    beepBtn.addEventListener("click", () => {
        beep();
    });
});
