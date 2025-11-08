// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
use std::{collections::HashMap, fs::File, io::Read};

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
    format!("Hello, {}! You've been greeted from Rust!", name)
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
            let (time, other) = line
                .strip_prefix('[')
                .and_then(|s| s.split_once(']'))
                .ok_or("invalid line format")?;

            // 再解析其他所需字段
            let mut words = other
                .split_whitespace()
                .filter_map(|word| {
                    if word.contains("head")
                        || word.contains("tail")
                        || word.contains("number")
                        || word.contains("height")
                        || word.contains("pos")
                    {
                        let (k, v) = word.split_once(':')?;
                        return Some((k.to_string(), v.to_string()));
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

    println!("train data len: {}", train_data.len());
    Ok(train_data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_data() {
        let path = "D:/project/pingche_jingtang/data/log/2025.10.31/pingche_log.5.log".to_string();
        let data = parse_data(path).unwrap();

        assert!(!data.is_empty());
    }
}

fn parse_time(time: &str) -> Result<DateTime<Utc>, String> {
    let native_time = NaiveDateTime::parse_from_str(time, "%Y-%m-%d %H:%M:%S%.3f")
        .map_err(|_| "failed to parse time")?;

    Ok(DateTime::from_naive_utc_and_offset(native_time, Utc))
}
