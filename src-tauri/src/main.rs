// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use defines::{QTSInteraction, QTSOSCType};
use tauri::async_runtime::block_on;
use tauri::{App, AppHandle, Manager};
use rosc::{OscPacket, OscType, OscMessage};
use std::collections::HashMap;
use std::fs;
use std::env;
use std::io::Read;
use std::net::{SocketAddrV4, UdpSocket};
use std::str::FromStr;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;
use std::path::PathBuf;
use rfd::FileDialog;
use rosc::encoder;

use poem::{
    handler, listener::TcpListener, post,
    Route, Server, web::Json
};
use gsi_cs2::{player::MatchStats, provider::Provider, Body};


use dns_lookup::lookup_host;

use reqwest;

mod gsi_cfg;
mod defines;
mod firmware;

static QTSHOCK_SHK_STRENGTH: Mutex<u8> = Mutex::new(10);
static QTSHOCK_VIB_STRENGTH: Mutex<u8> = Mutex::new(80);
static QTSHOCK_IP: Mutex<String> = Mutex::new(String::new());

static VRC_OSC_THREAD: Mutex<bool> = Mutex::new(true);
static VRC_OSC_SENDER: Mutex<Option<UdpSocket>> = Mutex::new(None);
static VRC_OSC_CANSHOCK: Mutex<bool> = Mutex::new(true);

static CS_GSI_THREAD: Mutex<bool> = Mutex::new(false);
static CS_CURRENT_DEATH_COUNT: Mutex<u16> = Mutex::new(0);



#[tauri::command]
async fn set_shock_strength(strength: u8) {
    *QTSHOCK_SHK_STRENGTH.lock().unwrap() = strength;
    let to_addr = match SocketAddrV4::from_str("127.0.0.1:9000") {
        Ok(addr) => {
            println!("Got proper address!");
            addr
        },
        Err(_) => {
            println!("FAILED TO GET ADDRESS!");
            return;
        }
    };
    let mutex_sock = VRC_OSC_SENDER.lock().unwrap();
    // switch view
    let msg_buf = encoder::encode(&OscPacket::Message(OscMessage {
        addr: "/avatar/parameters/QTS_IN_SHOCK_STRENGTH".to_string(),
        args: vec![OscType::Float(((*QTSHOCK_SHK_STRENGTH.lock().unwrap() - 1) as f32) / 100.0)],
    }))
    .unwrap();

    mutex_sock.as_ref().unwrap().send_to(&msg_buf, to_addr).unwrap();
}

#[tauri::command]
async fn set_vibrate_strength(strength: u8) {
    *QTSHOCK_VIB_STRENGTH.lock().unwrap() = strength;
    let to_addr = match SocketAddrV4::from_str("127.0.0.1:9000") {
        Ok(addr) => {
            println!("Got proper address!");
            addr
        },
        Err(_) => {
            println!("FAILED TO GET ADDRESS!");
            return;
        }
    };
    let mutex_sock = VRC_OSC_SENDER.lock().unwrap();
    // switch view
    let msg_buf = encoder::encode(&OscPacket::Message(OscMessage {
        addr: "/avatar/parameters/QTS_IN_VIBRATE_STRENGTH".to_string(),
        args: vec![OscType::Float(((*QTSHOCK_VIB_STRENGTH.lock().unwrap() - 1) as f32) / 100.0)],
    }))
    .unwrap();

    mutex_sock.as_ref().unwrap().send_to(&msg_buf, to_addr).unwrap();
}





async fn death_check(data: Json<gsi_cs2::Body>) {
    let player = match data.player.as_ref() {
        Some(plyr) => {
            plyr
        },
        None => {
            return;
        }
    };
    let match_stats = match player.match_stats.as_ref() {
        Some(p_stats) => {
            p_stats
        },
        None => {
            return;
        }
    };
    let provider = match data.provider.as_ref() {
        Some(p_provider) => {
            p_provider
        },
        None => {
            return;
        }
    };

    if provider.steam_id != player.steam_id.clone().unwrap() {
        return;
    }

    if *CS_CURRENT_DEATH_COUNT.lock().unwrap() > match_stats.deaths {
        *CS_CURRENT_DEATH_COUNT.lock().unwrap() = 0;
    }

    if *CS_CURRENT_DEATH_COUNT.lock().unwrap() < match_stats.deaths {
        *CS_CURRENT_DEATH_COUNT.lock().unwrap() = match_stats.deaths;
        println!("Player died!");
        let mut map = HashMap::new();
        map.insert("_type", 1);
        map.insert("Strength", 2);

        let _ = trigger_qtshock(0, QTSInteraction::SHOCK).await;
    }
}









#[derive(Clone, serde::Serialize)]
struct Payload {
    message: String
}

async fn trigger_qtshock(shocker: u8, interaction: QTSInteraction) -> Result<(), String>{
    match interaction {
        QTSInteraction::SHOCK => {
            let strength = QTSHOCK_SHK_STRENGTH.lock().unwrap().to_string().clone();
            shock(shocker, strength.as_str()).await;
            Ok(())
        },
        QTSInteraction::VIBRATE => {
            let strength = QTSHOCK_SHK_STRENGTH.lock().unwrap().to_string().clone();
            vibrate(shocker, strength.as_str()).await;
            Ok(())
        },
        QTSInteraction::BEEP => {
            beep(shocker).await;
            Ok(())
        }
    }
}

#[tauri::command]
fn create_cs_config(app: AppHandle) {
    let folder: Option<PathBuf> = FileDialog::new()
        .set_directory("/")
        .set_title("Select the Counter Strike 2 game directory")
        .pick_folder();

    let cs_path: PathBuf = match folder {
        Some(path) => path,
        None => {
            app.emit_all("cs-rust-event", Payload { message: "Setup failed. You must select your CS2 game directory to set up QTShock integration.".to_string() });
            return;
        }
    };

    let path_str = match cs_path.to_str() {
        Some(path) => path,
        None => {
            app.emit_all("cs-rust-event", Payload { message: "Setup failed. Something is wrong with the name of your selected directory.".to_string() });
            return;
        }
    };

    if !path_str.ends_with("Counter-Strike Global Offensive") {
        app.emit_all("cs-rust-event", Payload { message: "Setup failed. This is not the path to CS2..".to_string() });
        return;
    }

    let gsi_config: gsi_cfg::gsi_cfg = gsi_cfg::gsi_cfg {
        uri: "http://127.0.0.1:3005".to_string(),
        timeout: "1.0".to_string(),
        buffer: "0.0".to_string(),
        throttle: "0.0".to_string(),
        heartbeat: "60.0".to_string(),
        auth: gsi_cfg::gsi_auth { token: "TOKEN".to_string() },
        output: gsi_cfg::gsi_output { precision: "3".to_string(), precision_position: "1".to_string(), precision_vector: "3".to_string() },
        data: gsi_cfg::gsi_data {
            map_round_wins: "1".to_string(),
            map: "1".to_string(),
            player_id: "1".to_string(),
            player_match_stats: "1".to_string(),
            player_state: "1".to_string(),
            player_weapons: "1".to_string(),
            provider: "1".to_string(),
            round: "1".to_string(),
            allgrenades: "1".to_string(),
            allplayers_id: "1".to_string(),
            allplayers_match_stats: "1".to_string(),
            allplayers_position: "1".to_string(),
            allplayers_state: "1".to_string(),
            allplayers_weapons: "1".to_string(),
            bomb: "1".to_string(),
            phase_countdowns: "1".to_string(),
            player_position: "1".to_string()
        }

    };
    let gsi_config_data = match vdf_serde::to_string(&gsi_config) {
        Ok(data) => {
            println!("{}", data);
            data
        },
        Err(e) => {
            app.emit_all("cs-rust-event", Payload { message: "Setup failed. Something went wrong when creating the gsi config.".to_string() });
            println!("{}", e);
            return;
        }
    };
    fs::write(path_str.to_string() + "/game/csgo/cfg/gamestate_integration_qtshock.cfg", gsi_config_data);
}

#[tauri::command]
fn start_cs_listener(app: AppHandle, start: bool) {
    *CS_GSI_THREAD.lock().unwrap() = start;
    if !start {
        return;
    }
    let cloned_app = app.clone();
    let _new_thread = thread::spawn(|| {
        block_on(async {
            cs_thread(cloned_app).await;
        })
    });
    let _ = block_on(beep(0));
    let _ = app.emit_all("cs-rust-event", Payload { message: format!("Toggled CS2 integration ON").into() });
}

#[handler]
async fn cs_update(data: Json<gsi_cs2::Body>) {

    death_check(data.clone()).await;
}

async fn stop_gsi_thread(app: AppHandle) {
    loop {
        let keep_thread: bool = *CS_GSI_THREAD.lock().unwrap();
        if !keep_thread {
            break;
        }
    }
    app.emit_all("cs-rust-event", Payload { message: format!("Toggled CS2 integration OFF").into() });
}

async fn cs_thread(app: AppHandle) -> Result<(), std::io::Error>{
    tracing_subscriber::fmt::try_init();
    let gsi_webserver = Route::new().at("/", post(cs_update));
    let cloned_app = app.clone();
    let _server = match Server::new(TcpListener::bind("127.0.0.1:3005"))
        .run_with_graceful_shutdown(gsi_webserver,
        async move {
            stop_gsi_thread(cloned_app).await
        },
        Some(Duration::from_secs(5)))
        .await {
            Ok(()) => {
                
            },
            Err(e) => {
                let _ = app.emit_all("cs-rust-event", Payload { message: format!("Something went wrong when starting the CS2 integration. The port provided is already in use.").into() });
            }
        };
    Ok(())
}


#[tauri::command]
fn start_vrc_osc(app: AppHandle, start: bool) {
    *VRC_OSC_THREAD.lock().unwrap() = start;
    if !start {
        return;
    }
    let _new_thread = thread::spawn(|| {
        vrc_osc_thread(app);
    });
    let _new_thread = thread::spawn(|| {
        vrc_osc_send_thread();
    });
    let _ = block_on(beep(0));
}



async fn handle_packet(app: &AppHandle, packet: OscPacket) {
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
            let addr_parts: Vec<&str> = msg.addr.split("_").collect();
            let shocker_index: u8 = match addr_parts[1].parse::<u8>() {
                Ok(index) => index,
                Err(e) => {
                    app.emit_all("vrc-osc-event", Payload { message: format!("Invalid QTShock OSC data received. Bad invalid shocker index.").into() }).unwrap();
                    0
                }
            };
            let qt_osc_type: QTSOSCType = match QTSOSCType::from_str(addr_parts[2]) {
                Ok(osc_type) => osc_type,
                _ => {
                    app.emit_all("vrc-osc-event", Payload { message: format!("Invalid QTShock OSC data received. Bad command type.").into() }).unwrap();
                    return;
                }
            };

            let qt_osc_interaction: QTSInteraction = match QTSInteraction::from_str(addr_parts[3]) {
                Ok(osc_interaction) => osc_interaction,
                _ => {
                    app.emit_all("vrc-osc-event", Payload { message: format!("Invalid QTShock OSC data received. Bad interaction type.").into() }).unwrap();
                    return;
                }
            };

            match qt_osc_type {
                QTSOSCType::PUSH => {
                    match msg.args[0] {
                        OscType::Float(f) => {
                            if f > 0.8f32 && *VRC_OSC_CANSHOCK.lock().unwrap() == true {
                                *VRC_OSC_CANSHOCK.lock().unwrap() = false;
                                match trigger_qtshock(shocker_index, qt_osc_interaction).await {
                                    Ok(()) => {
                                        app.emit_all("vrc-osc-event", Payload { message: format!("Boop").into() }).unwrap();
                                    },
                                    _ => {
                                        app.emit_all("vrc-osc-event", Payload { message: format!("Something went wrong when triggering your QTShock.").into() }).unwrap();
                                    }

                                }
                            }
                            if f < 0.2f32 && *VRC_OSC_CANSHOCK.lock().unwrap() == false {
                                *VRC_OSC_CANSHOCK.lock().unwrap() = true;
                                app.emit_all("vrc-osc-event", Payload { message: format!("Unboop").into() }).unwrap();
                            }
                        },
                        _ => {
                            app.emit_all("vrc-osc-event", Payload { message: format!("Invalid QTShock OSC data received. Bad value type.").into() }).unwrap();
                        }
                    }
                },
                QTSOSCType::HIT => {
                    match msg.args[0] {
                        OscType::Bool(b) => {
                            app.emit_all("vrc-osc-event", Payload { message: format!("HIT!!!!!!!!!!!!!!!!!!!!").into() }).unwrap();
                            if b {
                                match trigger_qtshock(shocker_index, qt_osc_interaction).await {
                                    Ok(()) => {
                                    },
                                    _ => {
                                        app.emit_all("vrc-osc-event", Payload { message: format!("Something went wrong when triggering your QTShock.").into() }).unwrap();
                                    }
    
                                }
                            }
                        },
                        _ => {
                            app.emit_all("vrc-osc-event", Payload { message: format!("Invalid QTShock OSC data received. Bad value type.").into() }).unwrap();
                        }
                    }
                }
            }
        }
        OscPacket::Bundle(bundle) => {
            app.emit_all("vrc-osc-event", Payload { message: format!("VRC OSC bundle | {:?}", bundle).into() }).unwrap();

        }
    }
}

fn vrc_osc_send_thread() {
    let host_addr = match SocketAddrV4::from_str("127.0.0.1:7766") {
        Ok(addr) => {
            println!("Got proper address!");
            addr
        },
        Err(_) => {
            println!("FAILED TO GET ADDRESS!");
            return;
        }
    };
    let to_addr = match SocketAddrV4::from_str("127.0.0.1:9000") {
        Ok(addr) => {
            println!("Got proper address!");
            addr
        },
        Err(_) => {
            println!("FAILED TO GET ADDRESS!");
            return;
        }
    };
    let send_sock = UdpSocket::bind(host_addr).unwrap();
    let mut mutex_sock = VRC_OSC_SENDER.lock().unwrap();
    *mutex_sock = Some(send_sock);
    // switch view
    let msg_buf = encoder::encode(&OscPacket::Message(OscMessage {
        addr: "/avatar/parameters/QTS_IN_SHOCK_STRENGTH".to_string(),
        args: vec![OscType::Float(((*QTSHOCK_SHK_STRENGTH.lock().unwrap() - 1) as f32) / 100.0)],
    }))
    .unwrap();

    mutex_sock.as_ref().unwrap().send_to(&msg_buf, to_addr).unwrap();

}

fn vrc_osc_thread(app: AppHandle) {

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
            Ok((size, _addr)) => {
                let (_, packet) = rosc::decoder::decode_udp(&buf[..size]).unwrap();
                block_on(handle_packet(&app, packet));
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
            *QTSHOCK_IP.lock().unwrap() = ips[0].to_string();
            ips[0].to_string()
        },
        Err(err) => {
            format!("Failed to find a QTShock on the network! Error: {}", err.to_string()).to_string()
        }
    }
    
}

#[tauri::command]
async fn shock(shocker: u8, strength: &str) -> Result<String, String> {
    match strength.to_string().parse::<i16>() {
        Ok(i) => {
            if i < 1 || i > 99 {
                return Err("".to_string());
            }
            let params = [("shocker", shocker.to_string()), ("strength", strength.to_string())];
            let client = reqwest::Client::new();
            let _res = client.post(format!("http://{}/shock", QTSHOCK_IP.lock().unwrap()))
            .form(&params)
            .send().await
            .unwrap();
            Ok(format!("Shock was called with: {}", strength))
        },
        _ => {
            Err("BAD SHOCK CALL".to_string())
        }
    }
    
}

#[tauri::command]
async fn vibrate(shocker: u8, strength: &str) -> Result<String, String> {
    match strength.to_string().parse::<i16>() {
        Ok(i) => {
            if i < 1 || i > 99 {
                return Err("".to_string());
            }
            let params = [("shocker", shocker.to_string()), ("strength", strength.to_string())];
            let client = reqwest::Client::new();
            let _res = client.post(format!("http://{}/vibrate", QTSHOCK_IP.lock().unwrap()))
            .form(&params)
            .send().await
            .unwrap();
            Ok(format!("Vibrate was called with: {}", strength))
        },
        _ => {
            Err("BAD VIBRATE CALL".to_string())
        }
    }
}

#[tauri::command]
async fn beep(shocker: u8) -> Result<String, String> {
    let client = reqwest::Client::new();
    let params = [("shocker", shocker.to_string())];
    let _res = client.post(format!("http://{}/beep", QTSHOCK_IP.lock().unwrap()))
    .form(&params)
    .send().await
    .unwrap();
    Ok(format!("Beep was called"))
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::default().build())
        .invoke_handler(tauri::generate_handler![load_local_ip, firmware::firmware::flash_device_firmware, firmware::firmware::factory_reset_device, firmware::firmware::get_available_serial_devices, set_shock_strength, set_vibrate_strength, create_cs_config, start_cs_listener, start_vrc_osc, shock, vibrate, beep])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
