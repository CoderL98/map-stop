//! Shared CSV parsing for avoid-point imports (system + user uploads).

use serde::Serialize;

use crate::error::AppError;

/// Soft limit for CSV upload bodies (2 MiB).
pub const MAX_CSV_BYTES: usize = 2 * 1024 * 1024;

pub fn ensure_csv_size(bytes: &[u8]) -> Result<(), AppError> {
    if bytes.len() > MAX_CSV_BYTES {
        return Err(AppError::bad_request(
            format!("CSV 文件过大（最大 {} KB）", MAX_CSV_BYTES / 1024),
        ));
    }
    Ok(())
}

#[derive(Debug, Clone)]
pub struct ParsedRow {
    pub line: usize,
    pub name: String,
    pub lat: f64,
    pub lon: f64,
    pub radius_m: f64,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RowError {
    pub line: usize,
    pub message: String,
}

#[derive(Debug)]
pub struct ParseOutcome {
    pub rows: Vec<ParsedRow>,
    pub errors: Vec<RowError>,
}

fn strip_utf8_bom(text: &str) -> &str {
    text.strip_prefix('\u{feff}').unwrap_or(text)
}

fn validate_coords(lat: f64, lon: f64, radius_m: f64) -> Result<(), String> {
    if !(-90.0..=90.0).contains(&lat) || !(-180.0..=180.0).contains(&lon) {
        return Err("经纬度超出范围".into());
    }
    if radius_m <= 0.0 || radius_m > 50_000.0 {
        return Err("半径必须大于 0 且不超过 50000 米".into());
    }
    Ok(())
}

fn is_blank_record(rec: &csv::StringRecord) -> bool {
    rec.iter().all(|f| f.trim().is_empty())
}

fn col_index(headers: &csv::StringRecord, names: &[&str]) -> Option<usize> {
    headers.iter().position(|h| {
        let h = h.trim().trim_start_matches('\u{feff}');
        names.iter().any(|n| h.eq_ignore_ascii_case(n))
    })
}

/// Parse avoid-points CSV bytes into valid rows + per-row errors.
/// Supports UTF-8 BOM, Chinese/English header aliases, skips empty lines.
pub fn parse_avoid_points_csv(bytes: &[u8]) -> Result<ParseOutcome, AppError> {
    let text = String::from_utf8_lossy(bytes);
    let text = strip_utf8_bom(&text);
    let mut rdr = csv::ReaderBuilder::new()
        .flexible(true)
        .trim(csv::Trim::All)
        .from_reader(text.as_bytes());

    let headers = rdr
        .headers()
        .map_err(|e| AppError::bad_request(format!("CSV 表头错误: {e}")))?
        .clone();

    let i_name = col_index(&headers, &["名称", "name"])
        .ok_or_else(|| AppError::bad_request("缺少列：名称（或 name）"))?;
    let i_lat = col_index(&headers, &["纬度", "lat", "latitude"])
        .ok_or_else(|| AppError::bad_request("缺少列：纬度（或 lat）"))?;
    let i_lon = col_index(&headers, &["经度", "lon", "lng", "longitude"])
        .ok_or_else(|| AppError::bad_request("缺少列：经度（或 lon/lng）"))?;
    let i_r = col_index(&headers, &["半径米", "radius", "radius_m", "radius_meters"]);
    let i_note = col_index(&headers, &["备注", "note", "comment", "description"]);

    let mut rows = Vec::new();
    let mut errors = Vec::new();

    for (idx, rec) in rdr.records().enumerate() {
        let line = idx + 2; // 1-based data line (header is line 1)
        let rec = match rec {
            Ok(r) => r,
            Err(e) => {
                errors.push(RowError {
                    line,
                    message: format!("解析失败: {e}"),
                });
                continue;
            }
        };
        if is_blank_record(&rec) {
            continue;
        }

        let name = rec.get(i_name).unwrap_or("").trim().to_string();
        if name.is_empty() {
            errors.push(RowError {
                line,
                message: "名称为空".into(),
            });
            continue;
        }

        let lat: f64 = match rec.get(i_lat).unwrap_or("").trim().parse() {
            Ok(v) => v,
            Err(_) => {
                errors.push(RowError {
                    line,
                    message: "纬度无效".into(),
                });
                continue;
            }
        };
        let lon: f64 = match rec.get(i_lon).unwrap_or("").trim().parse() {
            Ok(v) => v,
            Err(_) => {
                errors.push(RowError {
                    line,
                    message: "经度无效".into(),
                });
                continue;
            }
        };

        let radius_raw = i_r
            .and_then(|i| rec.get(i))
            .map(|s| s.trim())
            .filter(|s| !s.is_empty());
        let radius: f64 = match radius_raw {
            None => 80.0,
            Some(s) => match s.parse() {
                Ok(v) => v,
                Err(_) => {
                    errors.push(RowError {
                        line,
                        message: "半径无效".into(),
                    });
                    continue;
                }
            },
        };

        if let Err(msg) = validate_coords(lat, lon, radius) {
            errors.push(RowError {
                line,
                message: msg,
            });
            continue;
        }
        let note = i_note
            .and_then(|i| rec.get(i))
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());

        rows.push(ParsedRow {
            line,
            name,
            lat,
            lon,
            radius_m: radius,
            note,
        });
    }

    Ok(ParseOutcome { rows, errors })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_chinese_headers_and_skips_blank() {
        let csv = "\u{feff}名称,纬度,经度,半径米,备注\n好点,30.25,120.15,80,注\n,,,\n坏点,999,120.15,80,\n";
        let out = parse_avoid_points_csv(csv.as_bytes()).unwrap();
        assert_eq!(out.rows.len(), 1);
        assert_eq!(out.rows[0].name, "好点");
        assert_eq!(out.errors.len(), 1);
        assert_eq!(out.errors[0].line, 4);
    }

    #[test]
    fn accepts_english_aliases() {
        let csv = "name,lat,lng,radius_m,note\nA,30.1,120.1,50,\n";
        let out = parse_avoid_points_csv(csv.as_bytes()).unwrap();
        assert_eq!(out.rows.len(), 1);
        assert!((out.rows[0].lon - 120.1).abs() < 1e-9);
    }
}
