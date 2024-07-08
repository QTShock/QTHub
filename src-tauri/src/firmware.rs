pub mod firmware {
    use std::io::{copy, Cursor};
    use std::time::Duration;
    use tempfile::{Builder, TempDir};
    use std::path::{Path, PathBuf};
    use std::{fs};
    use std::fs::{File};
    use espflash::{self, elf::ElfFirmwareImage, flasher::{FlashData, FlashSettings}};
    use serialport::{self, ErrorKind, SerialPort, SerialPortInfo, SerialPortType, UsbPortInfo};
    use tauri::{AppHandle, Manager};
    use espflash::connection::Connection;

    #[cfg(unix)]
    pub type Port = serialport::TTYPort;
    #[cfg(windows)]
    pub type Port = serialport::COMPort;



    // define the payload struct
    #[derive(Clone, serde::Serialize)]
    struct Payload {
        message: String,
        progress: usize
    }



    async fn create_binary_from_response(response: reqwest::Response, file_path: PathBuf) -> Option<String> {
        let file_path_str = match file_path.to_str() {
            Some(str) => {
                str
            },
            _ => {
                return Some(format!("<r>Failed to flash firmware! (Invalid new file path)</r>"));
            }
        };

        println!("{}", file_path_str);

        let mut new_file = match File::create(&file_path) {
            Ok(file) => {
                file
            },
            _ => {
                return Some(format!("<r>Failed to flash firmware! (Couldn't create file: {})</r>", file_path_str));
            }
        };

        let mut file_bytes = match response.bytes().await {
            Ok(bytes) => {
                Cursor::new(bytes)
            },
            _ => {
                return Some(format!("<r>Failed to flash firmware! (Couldn't read bytes from response for: {})</r>", file_path_str));
            }
        };


        match copy(&mut file_bytes, &mut new_file) {
            Ok(bytes_written) => {},
            _ => {
                return Some(format!("<r>Failed to flash firmware! (Couldn't write binary bytes to file '{}')</r>", file_path_str));
            }
        };
        None
    }

    async fn download_binaries(path: &PathBuf) -> Option<String> {
        let firmware_url = "https://qtshock.com/downloads/bin/firmware.elf";
        let bootloader_url = "https://qtshock.com/downloads/bin/bootloader.bin";
        let partitions_url = "https://qtshock.com/downloads/bin/partitions.bin";
        let firmware_response = match reqwest::get(firmware_url).await {
            Ok(response) => {
                response
            },
            _ => {
                return Some(format!("<r>Failed to flash firmware! (Couldn't fetch firmware binary from QTShock servers)</r>"));
            }
        };
        let bootloader_response = match reqwest::get(bootloader_url).await {
            Ok(response) => {
                response
            },
            _ => {
                return Some(format!("<r>Failed to flash firmware! (Couldn't fetch bootloader binary from QTShock servers)</r>"));
            }
        };
        let partitions_response = match reqwest::get(partitions_url).await {
            Ok(response) => {
                response
            },
            _ => {
                return Some(format!("<r>Failed to flash firmware! (Couldn't fetch partitions binary from QTShock servers)</r>"));
            }
        };

        let firmware_path = path.join("firmware.elf");
        match create_binary_from_response(firmware_response, firmware_path).await {
            Some(error) => {
                return Some(error);
            },
            None => {}
        };
        let bootloader_path = path.join("bootloader.bin");
        match create_binary_from_response(bootloader_response, bootloader_path).await {
            Some(error) => {
                return Some(error);
            },
            None => {}
        };
        let partitions_path = path.join("partitions.bin");
        match create_binary_from_response(partitions_response, partitions_path).await {
            Some(error) => {
                return Some(error);
            },
            None => {}
        };

        return None
    }

    // Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
    #[tauri::command]
    pub async fn flash_device_firmware(app: tauri::AppHandle, port_str: &str, source: &str) -> Result<String, tauri::Error> {
        app.emit_all("update-progress-text", Payload { message: format!("<y>Starting flash...</y>").into(), progress: 0 }).unwrap();

        let mut firmware_path: PathBuf = PathBuf::default();
        let mut elf_file: Option<Vec<u8>> = None;
        let mut bootloader_path: PathBuf = PathBuf::default();
        let mut partitions_path: PathBuf = PathBuf::default();

        let tmp_dir: TempDir = match Builder::new().prefix("qtstmp").tempdir() {
            Ok(dir) => {
                println!("Created tmp folder!");
                dir
            },
            _ => {
                println!("Failed to create tmp folder");
                return Ok(format!("<r>Failed to flash firmware! (Couldn't create temp directory)</r>"));
            }
        };

        match source {
            "server" => {
                let tmp_dir_path = tmp_dir.path().to_path_buf();
                app.emit_all("update-progress-text", Payload { message: format!("<bl>Downloading binaries...</bl>").into(), progress: 15 }).unwrap();

                match download_binaries(&tmp_dir_path).await {
                    Some(error) => {
                        return Ok(error);
                    },
                    None => {}
                };

                firmware_path = tmp_dir_path.join("firmware.elf");
                bootloader_path = tmp_dir_path.join("bootloader.bin");
                partitions_path = tmp_dir_path.join("partitions.bin");



            },
            "local" => {
                let current_dir = match std::env::current_dir() {
                    Ok(path) => {
                        path
                    },
                    _ => {
                        return Ok(format!("<r>Failed to flash firmware! (Couldn't fetch current directory)</r>"));
                    }
                };
                firmware_path = current_dir.join("bin/firmware.elf");
                bootloader_path = current_dir.join("bin/bootloader.bin");
                partitions_path = current_dir.join("bin/partitions.bin");
            },
            _ => {
                return Ok(format!("<r>Failed to flash firmware! (Invalid source)</r>"));
            }
        }

        elf_file = match fs::read(&firmware_path) {
            Ok(file) => {
                app.emit_all("update-progress-text", Payload { message: format!("<y>Read local firmware binary!</y>").into(), progress: 70 }).unwrap();
                Some(file)
            },
            _ => {
                println!("Something went wrong!");
                return Ok(format!("<r>Failed to flash firmware! (Local binary not found or invalid at '{}')</r>", firmware_path.to_str().unwrap()));
                None
            }
        };

        if (elf_file.is_none()) {
            return Ok(format!("<r>Failed to flash firmware! (File couldn't be loaded)</r>"));
        }

        let found_port: SerialPortInfo = match serialport::available_ports().unwrap().iter().find(|x| x.port_name == port_str) {
            Some(port_info) => {
                port_info.clone()
            },
            _ => {
                return Ok(format!("<r>Failed to flash firmware! (Port '{}' doesn't exist)</r>", port_str));
            }
        };

        let usb_port_info: UsbPortInfo = match found_port.port_type {
            serialport::SerialPortType::UsbPort(usb_info) => {
                app.emit_all("update-progress-text", Payload { message: format!("<y>Found USB port '{}'!</y>", port_str).into(), progress: 20 }).unwrap();
                usb_info
            },
            _ => {
                return Ok(format!("<r>Failed to flash firmware! (Invalid port '{}')</r>", port_str));
            }
        };


        let opened_port: Port = match serialport::new(&found_port.port_name, 115200)
        .flow_control(serialport::FlowControl::None)
        .open_native() {
            Ok(mut port) => {
                app.emit_all("update-progress-text", Payload { message: format!("<y>Opened connection on serial port '{}'!</y>", &found_port.port_name).into(), progress: 40 }).unwrap();
                port
            },
            Err(err) => {
                println!("{}", err.description);
                return Ok(format!("<r>Failed to flash firmware! (Couldn't open serial port '{}'. Is it already in use?</r>", &found_port.port_name));
            }
        };

        app.emit_all("update-progress-text", Payload { message: "<bl>Connecting to QTShock flash...</bl>".to_string(), progress: 45 }).unwrap();

        let mut flasher = match espflash::flasher::Flasher::connect(
            opened_port,
            usb_port_info,
            Some(460800),
            true,
            true,
            false,
            Some(espflash::targets::Chip::Esp32),
            espflash::connection::reset::ResetAfterOperation::HardReset,
            espflash::connection::reset::ResetBeforeOperation::DefaultReset) {
                Ok(fsr) => {
                    app.emit_all("update-progress-text", Payload { message: format!("<y>Set up flasher!</y>").into(), progress: 55 }).unwrap();
                    fsr
                },
                Err(err) => {
                    return Ok(format!("<r>Failed to flash firmware! ({})</r>", err));
                }
            };

        app.emit_all("update-progress-text", Payload { message: format!("<bl>Erasing existing flash...</bl>").into(), progress: 65 }).unwrap();

        let _ = match flasher.erase_flash() {
            Ok(_) => {
                app.emit_all("update-progress-text", Payload { message: format!("<y>Erased existing flash!</y>").into(), progress: 70 }).unwrap();
            },
            Err(err) => {
                return Ok(format!("<r>Failed to flash firmware! ({})</r>", err));
            }
        };
        
        let flash_settings: FlashSettings = FlashSettings::new(Some(espflash::flasher::FlashMode::Dio), Some(espflash::flasher::FlashSize::_4Mb), Some(espflash::flasher::FlashFrequency::_40Mhz));

        let flash_data: FlashData = match FlashData::new(
            Some(&bootloader_path),
            Some(&partitions_path),
            Some(0x8000),
            Some("app0".to_string()),
            flash_settings,
            1 * 100 + 1) {
                Ok(data) => {
                    app.emit_all("update-progress-text", Payload { message: format!("<y>Set up flash data!</y>").into(), progress: 85 }).unwrap();
                    data
                },
                _ => {
                    return Ok(format!("<r>Failed to flash firmware! (Bad flash data)</r>"));
                }
            };

            let chip_target = flasher.chip().into_target();
        
        let freq = match chip_target.crystal_freq(flasher.connection()) {
            Ok(frequency) => {
                app.emit_all("update-progress-text", Payload { message: format!("<y>Got crystal frequency!</y>").into(), progress: 100 }).unwrap();
                frequency
            },
            _ => {
                return Ok(format!("<r>Failed to flash firmware! (Couldn't fetch crystal frequency)</r>"));
            }
        };


        let flash_res = match flasher.load_elf_to_flash(&elf_file.unwrap(), flash_data, Some(&mut QTShockProgress::new(Some(app))), freq) {
            Ok(()) => {
                format!("<g>Successfully flashed firmware!</g>")
            },
            _ => {
                return Ok(format!("<r>Failed to flash firmware! (Flashing FAILED)</r>"));
            }
        };

        Ok(format!("<g>Successfully flashed firmware!</g>"))
    }

    #[tauri::command]
    pub async fn get_available_serial_devices() -> Result<String, tauri::Error> {
        let mut return_html: String = String::default();
        let mut usb_count : u32 = 0;
        match serialport::available_ports() {
            Ok(ports) => {
                for port in ports {
                    let port_name: &str = port.port_name.as_str();
                    match port.port_type {
                        SerialPortType::UsbPort(port_info) => {
                            usb_count += 1;
                            return_html.push_str(format!("<option value=\"{}\">{}</option>", port_name, port_name).as_str());
                        },
                        _ => {

                        }
                    }
                }
            },
            _ => {

                return_html = "<option value=\"NULL\" disabled>No devices found</option>".to_string();
            }
        }
        if (usb_count == 0) {
            return_html = "<option value=\"NULL\" disabled>No devices found</option>".to_string();
        }
        Ok(return_html)
    }

    #[derive(Default)]
    pub struct QTShockProgress {
        pub app: Option<tauri::AppHandle>,
        pub total_progress: usize
    }

    impl QTShockProgress {
        pub fn new(app: Option<tauri::AppHandle>) -> Self {
            Self { app, total_progress: 0 }
        }
    }

    impl espflash::flasher::ProgressCallbacks for QTShockProgress {
        fn init(&mut self, addr: u32, total: usize) {
            match &self.app {
                    Some(app) => {
                        self.total_progress = total;
                        app.emit_all("update-progress-text", Payload { message: format!("<bl>Started flashing firmware!</bl>").into(), progress: 10 }).unwrap();
                        println!("{} - PROGRESS BAR INIT | SIZE: {}", format!("{addr:#X}"), total);
                },
                None => {

                }
            }
        }

        fn update(&mut self, current: usize) {
            match &self.app {
                Some(app) => {
                    app.emit_all("update-progress-text", Payload { message: format!("<bl>Flashing firmware...</bl>").into(), progress: (current as f32/(self.total_progress as f32/100.0)) as usize }).unwrap();
                    println!("PROGRESS BAR UPDATE: {}", current);
                },
                None => {

                }
            }
        }

        fn finish(&mut self) {
            match &self.app {
                Some(app) => {
                    app.emit_all("update-progress-text", Payload { message: format!("<g>Finished segment!</g>").into(), progress: 100 }).unwrap();
                    println!("PROGRESS BAR FINISH");
                },
                None => {
                
                }
            }
        }
    }
}