use chrono::offset::Local;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::process::{exit, Command};
use std::time::Duration;

#[link(name = "X11")]
extern "C" {
    fn XOpenDisplay(screen: usize) -> usize;
    fn XDefaultRootWindow(display: usize) -> usize;
    fn XStoreName(display: usize, window: usize, name: *const u8) -> i32;
    fn XFlush(display: usize) -> i32;
}

fn read(p: impl AsRef<Path>) -> std::io::Result<String> {
    let mut file = File::open(p)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;
    Ok(content)
}

fn battery(id: u8) -> std::io::Result<String> {
    let mut bat_prefix: String = id.to_string();
    let bat_status = read(format!("/sys/class/power_supply/BAT{id}/status"))?;

    if bat_status.contains("Charging") {
        bat_prefix = String::from('C');
    }

    let cap = read(format!("/sys/class/power_supply/BAT{id}/capacity"))?;

    Ok(format!("{bat_prefix}: {cap}%"))
}

fn sound() -> std::io::Result<String> {
    let status = Command::new("amixer")
        .arg("get")
        .arg("Master")
        .output()?
        .stdout;

    let stdout = std::str::from_utf8(&status).unwrap();

    let mono = stdout.split('\n').collect::<Vec<&str>>()[4];
    let words = mono.split_whitespace().collect::<Vec<&str>>();
    let volume = words[3].replace(['[', ']'], "");
    let audible = words[5].replace(['[', ']'], "");

    if audible == "off" {
        return Ok(String::from("MUTED"));
    }

    Ok(volume)
}

fn backlight() -> std::io::Result<String> {
    let status = Command::new("xbacklight").output()?.stdout;
    let stdout = std::str::from_utf8(&status).unwrap();

    let percentage = stdout.split('.').next().unwrap().to_owned();

    Ok(format!("{percentage}%"))
}

fn has_commands(commands: &[&str]) -> bool {
    for c in commands {
        if Command::new(c).spawn().is_err() {
            return false;
        }
    }
    true
}

#[cfg(feature = "t480")]
fn get_items() -> std::io::Result<(String, String, String, String)> {
    let time = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let bat0 = battery(0)?;
    let bat1 = battery(1)?;
    let vol = sound()?;
    let brightness = backlight()?;
    Ok((vol, brightness, format!("{bat0} {bat1}"), time))
}

fn main() -> std::io::Result<()> {
    if !has_commands(&["xbacklight", "amixer"]) {
        eprintln!("`xbacklight` and `amixer` commands are required");
        exit(1)
    }

    let display = unsafe { XOpenDisplay(0) };
    let window = unsafe { XDefaultRootWindow(display) };

    loop {
        let (volume, brightness, battery, time) = get_items()?;
        let name =
            format!("[V: {volume}] [B: {brightness}] [{battery}] [{time}]\0").replace('\n', "");
        unsafe { XStoreName(display, window, name.as_ptr()) };
        unsafe { XFlush(display) };
        std::thread::sleep(Duration::from_secs(1));
    }
}
