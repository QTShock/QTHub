import { invoke } from "@tauri-apps/api/tauri";
import { listen } from "@tauri-apps/api/event";

let pageMsgEl: HTMLElement | null;
let mainPanelEl: HTMLElement | null;

let csToggle: HTMLInputElement | null;

let csConsoleDiv: HTMLElement | null;
let csConsole: HTMLElement | null;

let shockStrengthEl: HTMLInputElement | null;
let vibrateStrengthEl: HTMLInputElement | null;

let qtshockIp: string;





type Payload = {
    message: string;
  };
  
async function startCSEventListener() {
  await listen<Payload>('cs-rust-event', (event) => {
      console.log(event.payload.message);
      csLog(event.payload.message);
  });
}

export async function setupCSConfig() {
    await invoke("create_cs_config");
}

declare global {
    interface Window { setupCSConfig: any }
}

window.setupCSConfig = setupCSConfig;

function checkIpAddress(ip: string) { 
    const ipv4Pattern =  
        /^(\d{1,3}\.){3}\d{1,3}$/; 
    const ipv6Pattern =  
        /^([0-9a-fA-F]{1,4}:){7}[0-9a-fA-F]{1,4}$/; 
    return ipv4Pattern.test(ip) || ipv6Pattern.test(ip); 
}

async function setShockStrength(strength: number) {
    if (!shockStrengthEl) {
        return;
    }
    await invoke("set_shock_strength", {strength: strength })
}

async function setVibrateStrength(strength: number) {
    if (!vibrateStrengthEl) {
        return;
    }
    await invoke("set_vibrate_strength", {strength: strength })
}

async function csLog(txt: string) {
    if (!csConsole) return;
    if (!csConsoleDiv) return;
    let entries = csConsole.getElementsByTagName("li");
    let brEntries = csConsole.getElementsByTagName("br");
    if (entries.length > 50) {
        csConsole.removeChild(entries[0]);
        csConsole.removeChild(brEntries[0]);
    }
    csConsole.innerHTML = csConsole.innerHTML + `<li>-> ${txt}</li><br>`;
    csConsoleDiv.scrollTop = csConsoleDiv.scrollHeight;
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

    csToggle = document.getElementById("cs-toggle") as HTMLInputElement;
    if (!csToggle) return;
    
    csConsole = document.getElementById("cs-console") as HTMLElement;
    if (!csConsole) return;
    csConsoleDiv = csConsole.parentElement;
    startCSEventListener();
    
    csToggle.addEventListener("change", async (e) => {
        if (e.target) {
            let toggled = (e.target as HTMLInputElement).checked;
            await invoke("start_cs_listener", {start: toggled});
        }
    });

    shockStrengthEl = document.getElementById("shock-strength") as HTMLInputElement;
    shockStrengthEl.addEventListener("change", (e) => {
        if (e.target) {
            let strengthStr = (e.target as HTMLInputElement).value;
            let strength = parseInt(strengthStr);
            if (strength < 1 || strength > 99) {
                if (!shockStrengthEl) return;
                shockStrengthEl.value = "24";
                strength = 24;
                
            }
            setShockStrength(strength);
            
        }
    });
    vibrateStrengthEl = document.getElementById("vibrate-strength") as HTMLInputElement;
    vibrateStrengthEl.addEventListener("change", (e) => {
        if (e.target) {
            let strengthStr = (e.target as HTMLInputElement).value;
            let strength = parseInt(strengthStr);
            if (strength < 1 || strength > 99) {
                if (!vibrateStrengthEl) return;
                vibrateStrengthEl.value = "24";
                strength = 24;
            }
            setVibrateStrength(strength);
        }
    });
});
