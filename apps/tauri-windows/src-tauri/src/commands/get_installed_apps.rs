use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;
use tauri::command;

#[derive(Debug, Serialize, Deserialize)]
pub struct InstalledApp {
    name: String,
    publisher: Option<String>,
    version: Option<String>,
    install_location: Option<String>,
    image: Option<String>,
}

#[tauri::command]
pub fn get_installed_apps() -> Vec<InstalledApp> {
    let mut apps = Vec::new();

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;

        let mut cmd = Command::new("powershell");
        cmd.creation_flags(CREATE_NO_WINDOW);

        // PowerShell command to get installed apps from registry and their icon paths
        let output = cmd
            .args(&[
                "-Command",
                r#"
                Get-ItemProperty HKLM:\\Software\\Wow6432Node\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\*, 
                HKLM:\\Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\* | 
                Where-Object { 
                    $_.DisplayName -and (
                        $_.Publisher -match 'Adobe|Blender|Epic Games|Valve|Steam|Unity|Unreal|EA|Electronic Arts|Ubisoft|Riot Games|Battle.net|Bethesda|CD Projekt|2K Games|Rockstar Games|Square Enix|Activision|Microsoft Gaming' -or
                        $_.DisplayName -match 'Adobe|Blender|Steam|Epic Games|Unity|Unreal Engine|Origin|Battle.net|GOG|League of Legends|Valorant|Minecraft|Photoshop|Premiere|After Effects|Illustrator|InDesign|Maya|3ds Max|Cinema 4D|ZBrush|Substance|Houdini|Nuke|DaVinci'
                    )
                } |
                Select-Object DisplayName, Publisher, DisplayVersion, InstallLocation, DisplayIcon
                "#
            ])
            .output()
            .expect("Failed to execute PowerShell command");

        let output_str = String::from_utf8_lossy(&output.stdout);

        for line in output_str.lines().skip(3) {
            // Skip header lines
            let fields: Vec<&str> = line.split("  ").filter(|s| !s.trim().is_empty()).collect();
            if fields.len() >= 1 {
                let icon_path = fields
                    .get(4)
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty() && Path::new(s).exists());

                apps.push(InstalledApp {
                    name: fields[0].trim().to_string(),
                    publisher: fields.get(1).map(|s| s.trim().to_string()),
                    version: fields.get(2).map(|s| s.trim().to_string()),
                    install_location: fields.get(3).map(|s| s.trim().to_string()),
                    image: icon_path,
                });
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        // Filter for common game/creative apps on Linux
        let common_apps = [
            "steam",
            "epic-games-launcher",
            "blender",
            "gimp",
            "krita",
            "unity-editor",
            "godot",
            "adobe-*",
            "obs-studio",
        ];

        let output = Command::new("dpkg").args(&["--get-selections"]).output();

        if let Ok(output) = output {
            let output_str = String::from_utf8_lossy(&output.stdout);
            for line in output_str.lines() {
                let fields: Vec<&str> = line.split_whitespace().collect();
                if fields.len() >= 1 {
                    let name = fields[0].to_string();
                    if common_apps.iter().any(|&app| name.contains(app)) {
                        // Try to find icon in standard locations
                        let icon_path =
                            format!("/usr/share/icons/hicolor/256x256/apps/{}.png", name);
                        let icon = if Path::new(&icon_path).exists() {
                            Some(icon_path)
                        } else {
                            None
                        };

                        apps.push(InstalledApp {
                            name,
                            publisher: None,
                            version: None,
                            install_location: None,
                            image: icon,
                        });
                    }
                }
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        // Filter for common game/creative apps on macOS
        let common_apps = [
            "Adobe",
            "Blender",
            "Steam",
            "Epic Games Launcher",
            "Unity",
            "Unreal Engine",
            "Battle.net",
            "Photoshop",
            "Illustrator",
            "Final Cut Pro",
            "Logic Pro",
            "Maya",
            "ZBrush",
            "Cinema 4D",
            "DaVinci Resolve",
            "OBS",
        ];

        let output = Command::new("ls")
            .args(&["-1", "/Applications"])
            .output()
            .expect("Failed to execute ls command");

        let output_str = String::from_utf8_lossy(&output.stdout);
        for line in output_str.lines() {
            if line.ends_with(".app") {
                let app_name = line.replace(".app", "");
                if common_apps.iter().any(|&name| app_name.contains(name)) {
                    // Get app icon path from the .app bundle
                    let icon_path =
                        format!("/Applications/{}/Contents/Resources/AppIcon.icns", line);
                    let icon = if Path::new(&icon_path).exists() {
                        Some(icon_path)
                    } else {
                        None
                    };

                    apps.push(InstalledApp {
                        name: app_name,
                        publisher: None,
                        version: None,
                        install_location: Some(format!("/Applications/{}", line)),
                        image: icon,
                    });
                }
            }
        }
    }

    apps
}
