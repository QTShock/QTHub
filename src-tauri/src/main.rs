// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::{AppHandle, Manager};
use rosc::OscPacket;
use std::env;
use std::net::{SocketAddrV4, UdpSocket};
use std::str::FromStr;
use std::sync::Mutex;
use std::thread;

use dns_lookup::lookup_host;
use reqwest;


static VRC_OSC_THREAD: Mutex<bool> = Mutex::new(true);


#[derive(Clone, serde::Serialize)]
struct Payload {
    message: String
}

#[tauri::command]
fn start_vrc_osc(app: tauri::AppHandle, start: bool) {
    *VRC_OSC_THREAD.lock().unwrap() = start;
    if !start {
        return;
    }
    let new_thread = thread::spawn(|| {
        vrc_osc_thread(app);
    });
}

fn handle_packet(app: &tauri::AppHandle, packet: OscPacket) {
    let keep_thread: bool = *VRC_OSC_THREAD.lock().unwrap();
    if !keep_thread {
        return;
    }
    match packet {
        OscPacket::Message(msg) => {
            if !msg.addr.contains("QTS_") {
                return;
            }
            app.emit_all("vrc-osc-event", Payload { message: format!("VRC OSC msg | {}: {:?}", msg.addr, msg.args).into() }).unwrap();
        }
        OscPacket::Bundle(bundle) => {
            app.emit_all("vrc-osc-event", Payload { message: format!("VRC OSC bundle | {:?}", bundle).into() }).unwrap();

        }
    }
}

fn vrc_osc_thread(app: tauri::AppHandle) {
    let addr = match SocketAddrV4::from_str("127.0.0.1:9001") {
        Ok(addr) => {
            println!("Got proper address!");
            addr
        },
        Err(_) => {
            println!("FAILED TO GET ADDRESS!");
            return;
        }
    };
    let sock = UdpSocket::bind(addr).unwrap();
    app.emit_all("vrc-osc-event", Payload { message: format!("Listening to {}", addr).into() }).unwrap();
    println!("Listening to {}", addr);

    let mut buf = [0u8; rosc::decoder::MTU];

    loop {
        let keep_thread: bool = *VRC_OSC_THREAD.lock().unwrap();
        if !keep_thread {
            break;
        }
        match sock.recv_from(&mut buf) {
            Ok((size, addr)) => {
                let (_, packet) = rosc::decoder::decode_udp(&buf[..size]).unwrap();
                handle_packet(&app, packet);
            }
            Err(e) => {
                println!("Error receiving from socket: {}", e);
                break;
            }
        }
    }
    println!("VRC OSC Socket closed!");
    app.emit_all("vrc-osc-event", Payload { message: "VRC OSC Socket closed".into() }).unwrap();
}


#[tauri::command]
fn load_local_ip() -> String {
    let hostname = "qtshock.local";
    match lookup_host(hostname) {
        Ok(ips) => {
            ips[0].to_string()
        },
        Err(err) => {
            format!("Failed to find a QTShock on the network! Error: {}", err.to_string()).to_string()
        }
    }
    
}

#[tauri::command]
fn shock(ip: &str, strength: &str) -> String {
    match strength.to_string().parse::<i16>() {
        Ok(i) => {
            let params = [("strength", strength)];
            let client = reqwest::blocking::Client::new();
            let res = client.post(format!("http://{}/shock", ip))
            .form(&params)
            .send()
            .unwrap();
            format!("Shock was called with: {}", strength)
        },
        _ => {
            "BAD SHOCK CALL".to_string()
        }
    }
    
}

#[tauri::command]
fn vibrate(ip: &str, strength: &str) -> String {
    match strength.to_string().parse::<i16>() {
        Ok(i) => {
            if i < 1 || i > 99 {
                return "".to_string();
            }
            let params = [("strength", strength)];
            let client = reqwest::blocking::Client::new();
            let res = client.post(format!("http://{}/vibrate", ip))
            .form(&params)
            .send()
            .unwrap();
            format!("Vibrate was called with: {}", strength)
        },
        _ => {
            "BAD VIBRATE CALL".to_string()
        }
    }
}

#[tauri::command]
fn beep(ip: &str) -> String {
    let client = reqwest::blocking::Client::new();
    let res = client.post(format!("http://{}/beep", ip))
    .send()
    .unwrap();
    format!("Beep was called")
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![load_local_ip, start_vrc_osc, shock, vibrate, beep])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
