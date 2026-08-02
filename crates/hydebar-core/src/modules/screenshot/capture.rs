//! Taking the screenshots: slurp for the selection, grim for the capture.

use std::process::Command;

use log::{debug, error};

use super::{Screenshot, ScreenshotAction};

impl Screenshot {
    /// Take a screenshot with the specified action.
    ///
    /// The whole capture runs on its own thread: the area capture waits for
    /// the user to finish dragging the selection, and a bar that waited with
    /// it stopped repainting and answering for that whole time. Waiting for
    /// the capture tools on that thread also reaps them, so no capture leaves
    /// a zombie process behind.
    pub fn take_screenshot(&self, action: ScreenshotAction) {
        let screenshot_dir = dirs::picture_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("/tmp"))
            .join("Screenshots");

        if let Err(err) = std::fs::create_dir_all(&screenshot_dir) {
            error!("Failed to create screenshots directory: {err}");
            return;
        }

        let timestamp = chrono::Local::now().format("%Y-%m-%d_%H-%M-%S");
        let filename = screenshot_dir.join(format!("screenshot_{timestamp}.png"));

        std::thread::spawn(move || {
            let result = match action {
                ScreenshotAction::Area => {
                    debug!("Taking area screenshot");

                    match Command::new("slurp").output() {
                        Ok(output) if output.status.success() => {
                            let geometry =
                                String::from_utf8_lossy(&output.stdout).trim().to_string();

                            Command::new("grim")
                                .arg("-g")
                                .arg(geometry)
                                .arg(&filename)
                                .status()
                        }
                        Ok(_) => {
                            debug!("Slurp cancelled by user");
                            return;
                        }
                        Err(err) => {
                            error!("Failed to run slurp: {err}");
                            return;
                        }
                    }
                }
                ScreenshotAction::Window => {
                    debug!("Taking window screenshot (fullscreen for now)");
                    Command::new("grim").arg(&filename).status()
                }
                ScreenshotAction::Fullscreen => {
                    debug!("Taking fullscreen screenshot");
                    Command::new("grim").arg(&filename).status()
                }
            };

            match result {
                Ok(status) if status.success() => {
                    debug!("Screenshot saved to: {}", filename.display());
                }
                Ok(status) => error!("screenshot tool reported {status}"),
                Err(err) => error!("Failed to take screenshot: {err}")
            }
        });
    }
}
