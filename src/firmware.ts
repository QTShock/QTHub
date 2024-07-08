import { app } from "@tauri-apps/api"
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/tauri";
import { Store } from "tauri-plugin-store-api";

const store = new Store(".settings.dat");

let deviceSelectEl: HTMLSelectElement | null;
let refreshDevicesBtnEl: HTMLElement | null;
let statusLabelEl: HTMLElement | null;
let progressBarEl: HTMLProgressElement | null;
let progressLabelEl: HTMLElement | null;

type Payload = {
  message: string;
  progress: number;
};

async function startSerialEventListener() {
  await listen<Payload>('update-progress-text', (event) => {
    if (statusLabelEl && progressBarEl && progressLabelEl) {
      statusLabelEl.innerHTML = event.payload.message;
      progressBarEl.value = event.payload.progress;
      progressLabelEl.innerHTML = `${event.payload.progress}%`;
      console.log(`Progress: ${event.payload.progress}`);
    }
  });
}

async function flashFirmware(binSource: string) {
  if (statusLabelEl) {
    let portString = await store.get("selected-device") as string;
    console.log(portString);
    statusLabelEl.innerHTML = await invoke("flash_device_firmware", {app: app, portStr: portString, source: binSource}) as string;
  }
}

async function updateDevices() {
  console.log("Called updateDevices!");
  if (deviceSelectEl) {
    let serialDevicesHTML = await invoke("get_available_serial_devices") as string;
    deviceSelectEl.innerHTML = serialDevicesHTML;
    console.log(deviceSelectEl.innerHTML);
    if (!serialDevicesHTML.includes("No devices found")) {
      await store.set("selected-device", (deviceSelectEl as HTMLSelectElement).value);
    }
    if (deviceSelectEl) {
      let selectedDevice = await store.get("selected-device");
      if (selectedDevice) {
        deviceSelectEl.value = selectedDevice as string;
      }
    }
  }
}

window.addEventListener("DOMContentLoaded", async () => {
  deviceSelectEl = document.querySelector("#devices");
  refreshDevicesBtnEl = document.querySelector("#refresh-devices-btn");
  statusLabelEl = document.querySelector("#status-label");
  progressBarEl = document.querySelector("#progress-bar");
  progressLabelEl = document.querySelector("#progress-label");

  if (refreshDevicesBtnEl) {
    refreshDevicesBtnEl.onclick = updateDevices;
  }

  updateDevices();

  if (deviceSelectEl) {
    let selectedDevice = await store.get("selected-device");
    if (selectedDevice) {
      deviceSelectEl.value = selectedDevice as string;
    }
  }
  startSerialEventListener();
  document.querySelector("#update-form")?.addEventListener("submit", (e) => {
    e.preventDefault();
    let form: HTMLFormElement | null = document.querySelector("#update-form");
    if (form)
    {
      let formData = new FormData(form);
      let updateSource = formData.get("update-source") as string;
      flashFirmware(updateSource.toString());
    }
    
  });
  document.getElementById("devices")?.addEventListener("change", async (event) => {
    await store.set("selected-device", (event.target as HTMLSelectElement).value);
    console.log(`Switched managed device to ${await store.get("selected-device")}`);
    store.save();
  });
});
