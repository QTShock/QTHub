import { invoke } from "@tauri-apps/api/tauri";
import { listen } from "@tauri-apps/api/event";

let pageMsgEl: HTMLElement | null;
let mainPanelEl: HTMLElement | null;

let vrcToggle: HTMLInputElement | null;

let oscConsoleDiv: HTMLElement | null;
let oscConsole: HTMLElement | null;

let shockStrengthEl: HTMLInputElement | null;
let vibrateStrengthEl: HTMLInputElement | null;

let qtshockIp: string;





type Payload = {
    message: string;
  };
  
  async function startOscEventListener() {
    await listen<Payload>('vrc-osc-event', (event) => {
        console.log(event.payload.message);
        oscLog(event.payload.message);
    });
  }




function checkIpAddress(ip: string) { 
    const ipv4Pattern =  
        /^(\d{1,3}\.){3}\d{1,3}$/; 
    const ipv6Pattern =  
        /^([0-9a-fA-F]{1,4}:){7}[0-9a-fA-F]{1,4}$/; 
    return ipv4Pattern.test(ip) || ipv6Pattern.test(ip); 
} 

async function shock() {
  if (shockStrengthEl) {
    console.log(await invoke("shock", {ip: qtshockIp, strength: shockStrengthEl.value }));
  }
}

async function vibrate() {
    if (vibrateStrengthEl) {
      console.log(await invoke("vibrate", {ip: qtshockIp, strength: vibrateStrengthEl.value }));
    }
}

async function beep() {
    console.log(await invoke("beep", {ip: qtshockIp}));
}

async function oscLog(txt: string) {
    if (!oscConsole) return;
    if (!oscConsoleDiv) return;
    let entries = oscConsole.getElementsByTagName("li");
    let brEntries = oscConsole.getElementsByTagName("br");
    if (entries.length > 50) {
        oscConsole.removeChild(entries[0]);
        oscConsole.removeChild(brEntries[0]);
    }
    oscConsole.innerHTML = oscConsole.innerHTML + `<li>-> ${txt}</li><br>`;
    oscConsoleDiv.scrollTop = oscConsoleDiv.scrollHeight;
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

    vrcToggle = document.getElementById("vrc-toggle") as HTMLInputElement;
    if (!vrcToggle) return;
    
    oscConsole = document.getElementById("osc-console") as HTMLElement;
    if (!oscConsole) return;
    oscConsoleDiv = oscConsole.parentElement;
    startOscEventListener();
    
    vrcToggle.addEventListener("change", async (e) => {
        invoke("start_vrc_osc", {start: (e.target as HTMLInputElement).checked})
        await oscLog(`Toggled VRChat integration ${(e.target as HTMLInputElement).checked ? "ON" : "OFF"}`);
    });

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
});
