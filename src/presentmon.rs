use std::path::PathBuf;
use std::process::Command;
use std::sync::mpsc::{self, Receiver};

use eframe::egui;

pub(super) fn find() -> Option<PathBuf> {
    let mut candidates = Vec::new();
    if let Ok(exe) = std::env::current_exe()
        && let Some(folder) = exe.parent()
    {
        candidates.push(folder.join("PresentMon.exe"));
        candidates.push(folder.join("tools/PresentMon.exe"));
    }
    if let Some(paths) = std::env::var_os("PATH") {
        candidates.extend(std::env::split_paths(&paths).map(|path| path.join("PresentMon.exe")));
    }
    candidates.into_iter().find(|path| path.is_file())
}

pub(super) fn sample(
    executable: PathBuf,
    pid: u32,
    ctx: egui::Context,
) -> Receiver<Option<(f32, f32)>> {
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let mut command = Command::new(executable);
        command.args([
            "--process_id",
            &pid.to_string(),
            "--timed",
            "1",
            "--terminate_after_timed",
            "--output_stdout",
            "--no_console_stats",
            "--v1_metrics",
            "--exclude_dropped",
        ]);
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x08000000);
        }
        let sample = command
            .output()
            .ok()
            .and_then(|output| parse_csv(&String::from_utf8_lossy(&output.stdout)));
        let _ = sender.send(sample);
        ctx.request_repaint();
    });
    receiver
}

pub(super) fn parse_csv(value: &str) -> Option<(f32, f32)> {
    let mut lines = value.lines();
    let header = lines.find(|line| line.to_ascii_lowercase().contains("msbetweenpresents"))?;
    let columns: Vec<_> = header
        .split(',')
        .map(|value| value.trim().to_ascii_lowercase())
        .collect();
    let frame_index = columns
        .iter()
        .position(|name| name == "msbetweenpresents")?;
    let mut samples = Vec::new();
    for line in lines {
        let values: Vec<_> = line.split(',').collect();
        if let Some(frame_time) = values
            .get(frame_index)
            .and_then(|value| value.trim().parse::<f32>().ok())
            && frame_time.is_finite()
            && frame_time > 0.0
            && frame_time < 1000.0
        {
            samples.push(frame_time);
        }
    }
    if samples.is_empty() {
        return None;
    }
    let frame_time = samples.iter().sum::<f32>() / samples.len() as f32;
    Some((1000.0 / frame_time, frame_time))
}

#[cfg(test)]
mod tests {
    use super::parse_csv;

    #[test]
    fn frame_samples_produce_fps_and_frame_time() {
        let csv = "Application,ProcessID,msBetweenPresents,msBetweenDisplayChange\nGame.exe,42,16.0,16.1\nGame.exe,42,17.0,16.9\n";
        let (fps, frame_time) = parse_csv(csv).unwrap();
        assert!((frame_time - 16.5).abs() < 0.01);
        assert!((fps - 60.606).abs() < 0.01);
    }
}
