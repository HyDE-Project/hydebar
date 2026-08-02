//! Starting and stopping the screen recorder.

use std::process::Command;

use log::{debug, error};

use super::Screenshot;

impl Screenshot {
    /// Start screen recording.
    ///
    /// Creates the recordings directory first when it does not exist yet.
    pub fn start_recording(&mut self) {
        if self.is_recording {
            error!("Recording already in progress");
            return;
        }

        let video_dir = dirs::video_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("/tmp"))
            .join("Recordings");

        if let Err(err) = std::fs::create_dir_all(&video_dir) {
            error!("Failed to create recordings directory: {err}");
            return;
        }

        let timestamp = chrono::Local::now().format("%Y-%m-%d_%H-%M-%S");
        let filename = video_dir.join(format!("recording_{timestamp}.mp4"));

        debug!("Starting recording to: {}", filename.display());

        match Command::new("wf-recorder").arg("-f").arg(&filename).spawn() {
            Ok(mut child) => {
                self.recorder_pid = Some(child.id());
                self.is_recording = true;
                debug!("Recording started");

                std::thread::spawn(move || match child.wait() {
                    Ok(status) if status.success() => {}
                    Ok(status) => error!("the recorder ended with {status}"),
                    Err(err) => error!("waiting on the recorder failed: {err}")
                });
            }
            Err(err) => error!("Failed to start recording: {err}")
        }
    }

    /// Stop screen recording.
    pub fn stop_recording(&mut self) {
        if !self.is_recording {
            error!("No recording in progress");
            return;
        }

        debug!("Stopping recording");

        let Some(pid) = self.recorder_pid.take() else {
            self.is_recording = false;
            return;
        };

        match Command::new("kill")
            .arg("-INT")
            .arg(pid.to_string())
            .spawn()
        {
            Ok(mut child) => {
                std::thread::spawn(move || {
                    let _ = child.wait();
                });
                self.is_recording = false;
                debug!("Recording stopped");
            }
            Err(err) => error!("Failed to stop recording: {err}")
        }
    }
}
