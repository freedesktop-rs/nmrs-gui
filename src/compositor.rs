use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::process::Command;

#[derive(Clone, Debug)]
pub enum Compositor {
    Hyprland(PathBuf),
    Sway,
}

impl Compositor {
    pub fn detect() -> Option<Self> {
        if let Ok(signature) = std::env::var("HYPRLAND_INSTANCE_SIGNATURE") {
            let runtime = dirs::runtime_dir()?;
            let socket = runtime.join("hypr").join(signature).join(".socket.sock");
            return socket.exists().then_some(Self::Hyprland(socket));
        }
        if std::env::var_os("SWAYSOCK").is_some() {
            return Some(Self::Sway);
        }
        None
    }

    pub fn set_floating(
        &self,
        app_id: &str,
        floating: bool,
        size: (f32, f32),
    ) -> Result<(), String> {
        let (w, h) = (size.0.round() as i32, size.1.round() as i32);
        match self {
            Self::Hyprland(socket) => {
                let window = format!("class:^{app_id}$");
                let action = if floating { "enable" } else { "disable" };
                let lua = format!(
                    "dispatch hl.dsp.window.float({{ action = \"{action}\", window = \"{window}\" }})"
                );
                if hyprland(socket, &lua)? != "ok" {
                    let legacy = if floating { "setfloating" } else { "settiled" };
                    let reply = hyprland(socket, &format!("dispatch {legacy} {window}"))?;
                    if reply != "ok" {
                        return Err(reply);
                    }
                    if floating {
                        hyprland(
                            socket,
                            &format!("dispatch resizewindowpixel exact {w} {h},{window}"),
                        )?;
                    }
                    return Ok(());
                }
                if floating {
                    hyprland(
                        socket,
                        &format!(
                            "dispatch hl.dsp.window.resize({{ x = {w}, y = {h}, window = \"{window}\" }})"
                        ),
                    )?;
                }
                Ok(())
            }
            Self::Sway => {
                let command = if floating {
                    format!(
                        "[app_id=\"^{app_id}$\"] floating enable, resize set {w} {h}, move position center"
                    )
                } else {
                    format!("[app_id=\"^{app_id}$\"] floating disable")
                };
                let output = Command::new("swaymsg")
                    .arg(command)
                    .output()
                    .map_err(|e| e.to_string())?;
                if output.status.success() {
                    Ok(())
                } else {
                    Err(String::from_utf8_lossy(&output.stdout).trim().to_string())
                }
            }
        }
    }
}

fn hyprland(socket: &PathBuf, request: &str) -> Result<String, String> {
    let mut stream = UnixStream::connect(socket).map_err(|e| e.to_string())?;
    stream
        .write_all(request.as_bytes())
        .map_err(|e| e.to_string())?;
    let mut reply = String::new();
    stream
        .read_to_string(&mut reply)
        .map_err(|e| e.to_string())?;
    Ok(reply.trim().to_string())
}
