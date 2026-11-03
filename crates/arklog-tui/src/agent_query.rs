use std::fs::File;
use std::io::{self, BufReader, Read, Seek, SeekFrom};
use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::{DateTime, Local, NaiveDate, NaiveDateTime, TimeZone};
use regex::RegexBuilder;
use serde::{Deserialize, Deserializer, Serialize};

use crate::session_store::SessionDirectory;

#[derive(Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct LogQuery {
    #[serde(deserialize_with = "deserialize_time_bound")]
    pub start: Option<i64>,
    #[serde(deserialize_with = "deserialize_time_bound")]
    pub end: Option<i64>,
    pub regex: Option<String>,
    pub limit: usize,
    pub cursor: u64,
    pub session_id: Option<String>,
    pub until: Option<u64>,
}

fn deserialize_time_bound<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<i64>, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum TimeBound {
        Millis(i64),
        Text(String),
    }

    Option::<TimeBound>::deserialize(deserializer)?
        .map(|bound| match bound {
            TimeBound::Millis(millis) => Ok(millis),
            TimeBound::Text(text) => {
                parse_time_bound(text.trim()).map_err(serde::de::Error::custom)
            }
        })
        .transpose()
}

fn parse_time_bound(text: &str) -> Result<i64, &'static str> {
    if let Ok(millis) = text.parse::<i64>() {
        return Ok(millis);
    }
    if let Ok(time) = DateTime::parse_from_rfc3339(text) {
        return Ok(time.timestamp_millis());
    }
    let local = [
        "%Y-%m-%d %H:%M:%S%.f",
        "%Y/%m/%d %H:%M:%S%.f",
        "%Y-%m-%dT%H:%M:%S%.f",
        "%Y-%m-%d %H:%M",
        "%Y/%m/%d %H:%M",
        "%Y-%m-%dT%H:%M",
    ]
    .iter()
    .find_map(|format| NaiveDateTime::parse_from_str(text, format).ok())
    .or_else(|| {
        ["%Y-%m-%d", "%Y/%m/%d"].iter().find_map(|format| {
            NaiveDate::parse_from_str(text, format)
                .ok()?
                .and_hms_opt(0, 0, 0)
        })
    })
    .ok_or("invalid time: expected Unix millisecond integer, RFC3339, YYYY-MM-DD[ HH:mm[:ss[.fraction]]] or YYYY/MM/DD[ HH:mm[:ss[.fraction]]]")?;
    Local
        .from_local_datetime(&local)
        .single()
        .map(|time| time.timestamp_millis())
        .ok_or("ambiguous or nonexistent local time; supply RFC3339 with an explicit UTC offset")
}

impl Default for LogQuery {
    fn default() -> Self {
        Self {
            start: None,
            end: None,
            regex: None,
            limit: 100,
            cursor: 0,
            session_id: None,
            until: None,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LogExistsQuery {
    #[serde(default)]
    pub after: Option<i64>,
    pub regex: String,
}

#[derive(Debug, Serialize)]
pub struct LogRecord {
    pub index: u64,
    pub text: String,
}

#[derive(Debug, Serialize)]
pub struct LogQueryPage {
    pub session_id: String,
    pub raw_count: u64,
    pub records: Vec<LogRecord>,
    pub until: u64,
    pub next_cursor: Option<u64>,
}

// Files close before the directory guard, including on Windows after Clear.
pub struct LogSnapshot {
    pub(crate) data: File,
    pub(crate) index: File,
    pub(crate) raw_count: u64,
    pub(crate) directory: Arc<SessionDirectory>,
}

fn invalid(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message.into())
}

impl LogSnapshot {
    pub fn session_id(&self) -> String {
        self.directory
            .0
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned()
    }

    pub fn raw_count(&self) -> u64 {
        self.raw_count
    }

    pub fn query(mut self, query: &LogQuery) -> io::Result<LogQueryPage> {
        self.query_page(query, false)
    }

    pub fn has_logs(mut self, query: &LogExistsQuery) -> io::Result<bool> {
        let mut page_query = LogQuery {
            start: query.after,
            regex: Some(query.regex.clone()),
            limit: 1,
            session_id: Some(self.session_id()),
            until: Some(self.raw_count),
            ..LogQuery::default()
        };
        let started = Instant::now();
        loop {
            let page = self.query_page(&page_query, true)?;
            if !page.records.is_empty() {
                return Ok(true);
            }
            let Some(cursor) = page.next_cursor else {
                return Ok(false);
            };
            if started.elapsed() >= Duration::from_secs(5) {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "existence scan timed out; result is unknown",
                ));
            }
            page_query.cursor = cursor;
        }
    }

    fn query_page(&mut self, query: &LogQuery, exists_only: bool) -> io::Result<LogQueryPage> {
        if !(1..=1000).contains(&query.limit) {
            return Err(invalid("limit must be between 1 and 1000"));
        }
        if query
            .session_id
            .as_ref()
            .is_some_and(|id| id != &self.session_id())
            || (query.cursor > 0 && query.session_id.is_none())
        {
            return Err(invalid(
                "session changed or missing session_id; restart query",
            ));
        }
        let until = query.until.unwrap_or(self.raw_count);
        if query.cursor > until || until > self.raw_count {
            return Err(invalid("cursor/until outside session"));
        }
        let start = query.start;
        let end = query.end;
        if start.zip(end).is_some_and(|(start, end)| start > end) {
            return Err(invalid("start must be <= end"));
        }
        let regex = query
            .regex
            .as_deref()
            .map(|pattern| {
                if pattern.len() > 4096 {
                    return Err(invalid("regex exceeds 4096 bytes"));
                }
                RegexBuilder::new(pattern)
                    .case_insensitive(true)
                    .size_limit(1024 * 1024)
                    .dfa_size_limit(1024 * 1024)
                    .build()
                    .map_err(|error| invalid(error.to_string()))
            })
            .transpose()?;
        let mut page = LogQueryPage {
            session_id: self.session_id(),
            raw_count: self.raw_count,
            records: Vec::new(),
            until,
            next_cursor: None,
        };
        let mut data_file = &self.data;
        if query.cursor < until {
            let mut index_file = &self.index;
            index_file.seek(SeekFrom::Start(query.cursor * 8))?;
            let mut offset = [0; 8];
            index_file.read_exact(&mut offset)?;
            data_file.seek(SeekFrom::Start(u64::from_le_bytes(offset)))?;
        }
        let mut data = BufReader::new(data_file);
        let scan_end = until.min(query.cursor.saturating_add(100_000));
        let mut next = query.cursor;
        let mut bytes = 0;
        let mut scanned_bytes = 0;
        let started = Instant::now();
        for index in query.cursor..scan_end {
            if scanned_bytes >= 8 * 1024 * 1024
                || (next > query.cursor && started.elapsed() >= Duration::from_millis(250))
            {
                break;
            }
            let mut length = [0; 8];
            data.read_exact(&mut length)?;
            let length = u64::from_le_bytes(length);
            if length > 1024 * 1024 {
                return Err(invalid(format!(
                    "record {index} exceeds 1 MiB query record budget"
                )));
            }
            let mut record = vec![0; length as usize];
            data.read_exact(&mut record)?;
            let text = String::from_utf8(record).map_err(|e| invalid(e.to_string()))?;
            scanned_bytes += length + 8;
            next = index + 1;
            if start.is_some() || end.is_some() {
                let Some(time) = log_time(&text) else {
                    continue;
                };
                if start.is_some_and(|start| time < start || (exists_only && time == start))
                    || end.is_some_and(|end| time > end)
                {
                    continue;
                }
            }
            if regex.as_ref().is_none_or(|regex| regex.is_match(&text)) {
                if exists_only {
                    page.records.push(LogRecord {
                        index,
                        text: String::new(),
                    });
                    break;
                }
                // Bound retained output without dropping a record at the byte boundary.
                if bytes + text.len() > 256 * 1024 {
                    if page.records.is_empty() {
                        return Err(invalid(
                            "single matching record exceeds 256 KiB response budget",
                        ));
                    }
                    next = index;
                    break;
                }
                bytes += text.len();
                page.records.push(LogRecord { index, text });
                if page.records.len() == query.limit {
                    break;
                }
            }
        }
        page.next_cursor = (next < until).then_some(next);
        Ok(page)
    }
}

fn log_time(text: &str) -> Option<i64> {
    parse_epoch_millis(text.split_whitespace().next()?)
}

// HiLog `-v epoch` prints Unix seconds with three decimal places. Use integer
// arithmetic so boundary comparisons never lose a millisecond to float rounding.
pub(crate) fn parse_epoch_millis(token: &str) -> Option<i64> {
    let (seconds, fraction) = token.split_once('.')?;
    let digits = seconds.strip_prefix('-').unwrap_or(seconds);
    if digits.is_empty()
        || !digits.bytes().all(|byte| byte.is_ascii_digit())
        || fraction.len() != 3
        || !fraction.bytes().all(|byte| byte.is_ascii_digit())
    {
        return None;
    }
    let seconds = seconds.parse::<i128>().ok()?;
    let fraction = fraction.parse::<i128>().ok()?;
    let base = seconds.checked_mul(1000)?;
    let millis = if token.starts_with('-') {
        base.checked_sub(fraction)?
    } else {
        base.checked_add(fraction)?
    };
    i64::try_from(millis).ok()
}
