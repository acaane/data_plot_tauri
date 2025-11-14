// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
use std::{collections::HashMap, fs::{self, File}, io::Read};

use chrono::{DateTime, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
struct TrainInfo {
    time: DateTime<Utc>,
    number: String,
    head: f64,
    tail: f64,
    height: f64,
    pos: f64,
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {name}! You've been greeted from Rust!")
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![greet, parse_data,])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[tauri::command]
fn parse_data(path: String) -> Result<HashMap<String, Vec<TrainInfo>>, String> {
    let mut file = File::open(&path).map_err(|e| e.to_string())?;
    let mut buf = String::new();
    file.read_to_string(&mut buf).map_err(|e| e.to_string())?;

    let mut data = HashMap::new();
    for line in buf.lines() {
        if line.contains("train")
            && line.contains("head")
            && line.contains("tail")
            && line.contains("number")
        {
            // 先解析时间
            let (time, message) = line
                .strip_prefix('[')
                .and_then(|s| s.split_once(']'))
                .ok_or("invalid line format")?;

            // 再解析其他所需字段
            let mut words = message
                .split_whitespace()
                .filter_map(|word| {
                    if word.contains("head")
                        || word.contains("tail")
                        || word.contains("number")
                        || word.contains("height")
                        || word.contains("pos")
                    {
                        let (k, v) = word.split_once(':')?;
                        Some((k.to_string(), v.to_string()))
                    } else {
                        None
                    }
                })
                .collect::<HashMap<_, _>>();
            words.insert("time".to_string(), time.to_string());

            let number = words.get("number").ok_or("Missing train number")?.clone();
            data.entry(number).or_insert(Vec::new()).push(words);
        }
    }

    // info!("all data: {data:#?}");

    // 显示数据
    let train_data = data
        .into_iter()
        .map(|(num, vec)| {
            let mut infos = vec
                .into_iter()
                .map(|hash| {
                    Ok(TrainInfo {
                        time: parse_time(hash.get("time").ok_or("Missing time")?)?,
                        number: hash.get("number").ok_or("Missing number")?.clone(),
                        head: hash
                            .get("head")
                            .ok_or("Missing head")?
                            .parse::<f64>()
                            .map_err(|_| "Invalid head")?,
                        tail: hash
                            .get("tail")
                            .ok_or("Missing tail")?
                            .parse::<f64>()
                            .map_err(|_| "Invalid tail")?,
                        height: hash
                            .get("height")
                            .ok_or("Missing height")?
                            .parse::<f64>()
                            .map_err(|_| "Invalid height")?,
                        pos: hash
                            .get("pos")
                            .ok_or("Missing pos")?
                            .parse::<f64>()
                            .map_err(|_| "Invalid pos")?,
                    })
                })
                .collect::<Result<Vec<_>, String>>()?;

            infos.sort_by(|a, b| a.time.cmp(&b.time));

            Ok((num, infos))
        })
        .collect::<Result<HashMap<_, _>, String>>()?;

    Ok(train_data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_data() {
        let path = "../test_files/2025.10.31/pingche_log.5.log".to_string();
        let data = parse_data(path).unwrap();

        assert!(!data.is_empty());
    }

    #[test]
    fn test_parse_mupian_data() {
        let path = "../test_files/2025.11.13/unload_log2025.11.13.log".to_string();
        let data = parse_mupian_data(path).unwrap();

        assert!(!data.is_empty());
    }
}

fn parse_time(time: &str) -> Result<DateTime<Utc>, String> {
    let native_time = NaiveDateTime::parse_from_str(time, "%Y-%m-%d %H:%M:%S%.3f")
        .map_err(|_| "failed to parse time")?;

    Ok(DateTime::from_naive_utc_and_offset(native_time, Utc))
}

#[derive(Serialize, Deserialize, Debug)]
struct CpuInfo {
    time: DateTime<Utc>,
    cpu_usage: f32,
    mem_usage: f32,
    mem_used: f32,
}

impl CpuInfo {
    fn new() -> Self {
        Self {
            time: Utc::now(),
            cpu_usage: 0.0,
            mem_usage: 0.0,
            mem_used: 0.0,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct DirectionMismatchInfo {
    time: DateTime<Utc>,
    direction: i32,
    name: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct ReplenishInfo {
    time: DateTime<Utc>,
    // checked: Option<String>,
    name: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct MupianInfo {
    cpu_info: Option<CpuInfo>,
    direction_mismatch_info: Option<DirectionMismatchInfo>,
    replenish_info: Option<ReplenishInfo>,
}

impl MupianInfo {
    fn new() -> Self {
        Self {
            cpu_info: None,
            direction_mismatch_info: None,
            replenish_info: None,
        }
    }
}

#[tauri::command]
fn parse_mupian_data(path: String) -> Result<HashMap<String, Vec<MupianInfo>>, String> {
    let mut file= File::open(path).map_err(|e| e.to_string())?;
    let mut buf = String::new();
    file.read_to_string(&mut buf).map_err(|e| e.to_string())?;

    let mut data = HashMap::new();
    data.insert("".to_string(), vec![MupianInfo::new()]);
    let direction_mismatch_chekced = false;
    let replenish_info_written = "";

    let mut cpu_info = None;
    let mut direction_mismatch_info = None;
    let mut replenish_info = None;

    for line in buf.lines().take(1000) {
        if line.contains("cpu usage")
            || (line.contains("unload direction") && line.contains("match with"))
            || line.contains("send replenish finish")
        {
            let parts = line.splitn(4, ']').collect::<Vec<_>>();
            let time = parts[0].strip_prefix('[').ok_or("Missing time in line")?.trim();
            let _level = parts[1].strip_prefix(" [").ok_or("Missing level in line")?.trim();
            let name = parts[2].strip_prefix(" [").ok_or("Missing name in line")?.trim();
            let message = parts[3].trim();

            let mupian_info = 
            if message.contains("cpu usage") {
                let cpu_info = parse_cpu_info(message)?;
                (Some(cpu_info), None, None)
            } else if message.contains("unload direction") {
                (None, None, None)
            } else {
                (None, None, None)
            };

        }
    }

    Ok(data)    
}

fn parse_cpu_info(message: &str) -> Result<CpuInfo, String> {
    let mut parts = message.split(", ");
    let mut parse = |suffix: &str| {
        parts
           .next()
           .and_then(|s| s.split_once(':').map(|(_, v)| v.trim_end_matches(suffix)))
           .and_then(|v| v.parse::<f32>().ok()).ok_or("parse cpu info failed")
    };

    let cpu_usage = parse("%")?;
    let mem_usage = parse("%")?;
    let _total = parts.next()?;  // 不需要总内存数据，直接丢弃
    let mem_used = parse("MB")?;

    Ok(CpuInfo {
        time: Utc::now(),
        cpu_usage,
        mem_usage,
        mem_used,
    })
}

fn parse_direction_mismatch_info(message: &str) -> Result<DirectionMismatchInfo, String> {
    let direction = message
        .strip_prefix("unload direction ")
        .map(|s| {
            let word = s.split_whitespace();
            word
                .next()
                // .ok_or(format!("no word in: {message}"))?
                .and_then(|s| s.parse::<i32>().map_err(|e| e.to_string()))
        });
    // let name = direction == 
    
    // Ok(DirectionMismatchInfo {
    //     time: Utc::now(),
    //     direction: direction?,
    //     name: 
    // })
}
