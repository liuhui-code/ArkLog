use std::fs::{self, File, OpenOptions};
use std::io::{self, BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use regex::{Regex, RegexBuilder};

static NEXT_SESSION_ID: AtomicU64 = AtomicU64::new(1);
const MAX_REGEX_BYTES: usize = 4 * 1024;
const REGEX_COMPILED_LIMIT: usize = 1024 * 1024;
const FIND_COMPILED_LIMIT: usize = 512 * 1024;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct QueryRebuildWork {
    pub raw_records_scanned: u64,
    pub visible_records_scanned: u64,
}

pub struct SessionLogStore {
    directory: PathBuf,
    data_writer: Option<BufWriter<File>>,
    data_reader: Option<File>,
    raw_index_writer: Option<BufWriter<File>>,
    raw_index_reader: Option<File>,
    visible_index_writer: Option<BufWriter<File>>,
    visible_index_reader: Option<File>,
    find_index_writer: Option<BufWriter<File>>,
    find_index_reader: Option<File>,
    data_bytes: u64,
    raw_count: u64,
    visible_count: u64,
    find_count: u64,
    filter: Option<Regex>,
    filter_valid: bool,
    find_query: String,
    find: Option<Regex>,
    last_rebuild_work: QueryRebuildWork,
}

impl SessionLogStore {
    pub fn new() -> io::Result<Self> {
        let directory = create_session_directory()?;
        let (data_writer, data_reader) = create_file_pair(directory.join("raw.log"))?;
        let (raw_index_writer, raw_index_reader) = create_file_pair(directory.join("raw.idx"))?;
        let (visible_index_writer, visible_index_reader) =
            create_file_pair(directory.join("visible.idx"))?;
        let (find_index_writer, find_index_reader) = create_file_pair(directory.join("find.idx"))?;
        Ok(Self {
            directory,
            data_writer: Some(data_writer),
            data_reader: Some(data_reader),
            raw_index_writer: Some(raw_index_writer),
            raw_index_reader: Some(raw_index_reader),
            visible_index_writer: Some(visible_index_writer),
            visible_index_reader: Some(visible_index_reader),
            find_index_writer: Some(find_index_writer),
            find_index_reader: Some(find_index_reader),
            data_bytes: 0,
            raw_count: 0,
            visible_count: 0,
            find_count: 0,
            filter: None,
            filter_valid: true,
            find_query: String::new(),
            find: None,
            last_rebuild_work: QueryRebuildWork::default(),
        })
    }

    pub fn append_lines<I, S>(&mut self, lines: I) -> io::Result<()>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        for line in lines {
            self.append_line(line.as_ref())?;
        }
        Ok(())
    }

    pub fn raw_count(&self) -> u64 {
        self.raw_count
    }

    pub fn visible_count(&self) -> u64 {
        self.visible_count
    }

    pub fn set_filter(&mut self, query: &str) -> Result<(), String> {
        self.last_rebuild_work = QueryRebuildWork::default();
        let unchanged = self.filter_valid
            && self
                .filter
                .as_ref()
                .map_or(query.is_empty(), |filter| filter.as_str() == query);
        if unchanged {
            return Ok(());
        }
        if query.len() > MAX_REGEX_BYTES {
            self.invalidate_filter()?;
            return Err(format!(
                "Regular expression exceeds {MAX_REGEX_BYTES} bytes"
            ));
        }
        let filter = if query.is_empty() {
            None
        } else {
            match RegexBuilder::new(query)
                .case_insensitive(true)
                .size_limit(REGEX_COMPILED_LIMIT)
                .dfa_size_limit(REGEX_COMPILED_LIMIT)
                .build()
            {
                Ok(filter) => Some(filter),
                Err(error) => {
                    self.invalidate_filter()?;
                    return Err(error.to_string());
                }
            }
        };
        self.filter = filter;
        self.filter_valid = true;
        self.rebuild_query_indexes()
            .map_err(|error| error.to_string())
    }

    pub fn set_find(&mut self, query: &str) -> io::Result<()> {
        self.last_rebuild_work = QueryRebuildWork::default();
        if self.find_query == query {
            return Ok(());
        }
        if query.len() > MAX_REGEX_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("Find query exceeds {MAX_REGEX_BYTES} bytes"),
            ));
        }
        self.find = if query.is_empty() {
            None
        } else {
            Some(
                RegexBuilder::new(&regex::escape(query))
                    .case_insensitive(true)
                    .size_limit(FIND_COMPILED_LIMIT)
                    .dfa_size_limit(FIND_COMPILED_LIMIT)
                    .build()
                    .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))?,
            )
        };
        self.find_query = query.to_string();
        self.rebuild_find_index()
    }

    pub fn last_rebuild_work(&self) -> QueryRebuildWork {
        self.last_rebuild_work
    }

    pub fn find_count(&self) -> u64 {
        self.find_count
    }

    pub(crate) fn active_filter(&self) -> Option<&Regex> {
        self.filter_valid.then_some(self.filter.as_ref()).flatten()
    }

    pub fn find_visible_index(&mut self, ordinal: u64) -> io::Result<Option<u64>> {
        if ordinal >= self.find_count {
            return Ok(None);
        }
        self.find_index_writer
            .as_mut()
            .expect("find index writer")
            .flush()?;
        read_u64(
            self.find_index_reader.as_mut().expect("find index reader"),
            ordinal * 8,
        )
        .map(Some)
    }

    pub fn visible_window(&mut self, start: u64, limit: usize) -> io::Result<Vec<String>> {
        self.flush()?;
        let available = self.visible_count.saturating_sub(start).min(limit as u64) as usize;
        let mut lines = Vec::with_capacity(available);
        for visible_index in start..start + available as u64 {
            let raw_index = read_u64(
                self.visible_index_reader
                    .as_mut()
                    .expect("visible index reader"),
                visible_index * 8,
            )?;
            let data_offset = read_u64(
                self.raw_index_reader.as_mut().expect("raw index reader"),
                raw_index * 8,
            )?;
            lines.push(read_record(
                self.data_reader.as_mut().expect("raw data reader"),
                data_offset,
            )?);
        }
        Ok(lines)
    }

    pub fn clear(&mut self) -> io::Result<()> {
        let mut replacement = Self::new()?;
        replacement.filter = self.filter.clone();
        replacement.filter_valid = self.filter_valid;
        replacement.find_query.clone_from(&self.find_query);
        replacement.find.clone_from(&self.find);
        let old_session = std::mem::replace(self, replacement);
        drop(old_session);
        Ok(())
    }

    pub fn retained_heap_bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + self.directory.to_string_lossy().len()
            + self.find_query.capacity()
            + self.filter.as_ref().map_or(0, |_| REGEX_COMPILED_LIMIT)
            + self.find.as_ref().map_or(0, |_| FIND_COMPILED_LIMIT)
            + self.data_writer.as_ref().map_or(0, BufWriter::capacity)
            + self
                .raw_index_writer
                .as_ref()
                .map_or(0, BufWriter::capacity)
            + self
                .visible_index_writer
                .as_ref()
                .map_or(0, BufWriter::capacity)
            + self
                .find_index_writer
                .as_ref()
                .map_or(0, BufWriter::capacity)
    }

    fn append_line(&mut self, line: &str) -> io::Result<()> {
        let data_offset = self.data_bytes;
        let data = self.data_writer.as_mut().expect("raw data writer");
        data.write_all(&(line.len() as u64).to_le_bytes())?;
        data.write_all(line.as_bytes())?;
        append_u64(
            self.raw_index_writer.as_mut().expect("raw index writer"),
            data_offset,
        )?;
        if self.filter_matches(line) {
            let visible_index = self.visible_count;
            append_u64(
                self.visible_index_writer
                    .as_mut()
                    .expect("visible index writer"),
                self.raw_count,
            )?;
            self.visible_count += 1;
            if self.find_matches(line) {
                append_u64(
                    self.find_index_writer.as_mut().expect("find index writer"),
                    visible_index,
                )?;
                self.find_count += 1;
            }
        }
        self.data_bytes += 8 + line.len() as u64;
        self.raw_count += 1;
        Ok(())
    }

    fn rebuild_query_indexes(&mut self) -> io::Result<()> {
        self.flush()?;
        let filter = self.filter.clone();
        let filter_valid = self.filter_valid;
        let find = self.find.clone();
        let visible_writer = self
            .visible_index_writer
            .as_mut()
            .expect("visible index writer");
        visible_writer.get_ref().set_len(0)?;
        visible_writer.seek(SeekFrom::Start(0))?;
        let find_writer = self.find_index_writer.as_mut().expect("find index writer");
        find_writer.get_ref().set_len(0)?;
        find_writer.seek(SeekFrom::Start(0))?;
        let mut visible_count = 0;
        let mut find_count = 0;

        let data_reader = self.data_reader.as_mut().expect("raw data reader");
        data_reader.seek(SeekFrom::Start(0))?;
        let mut data_reader = BufReader::with_capacity(64 * 1024, data_reader);
        for raw_index in 0..self.raw_count {
            let line = read_next_record(&mut data_reader)?;
            if filter_valid && filter.as_ref().is_none_or(|filter| filter.is_match(&line)) {
                append_u64(visible_writer, raw_index)?;
                if find.as_ref().is_some_and(|find| find.is_match(&line)) {
                    append_u64(find_writer, visible_count)?;
                    find_count += 1;
                }
                visible_count += 1;
            }
        }
        visible_writer.flush()?;
        find_writer.flush()?;
        self.visible_count = visible_count;
        self.find_count = find_count;
        self.last_rebuild_work.raw_records_scanned = self.raw_count;
        Ok(())
    }

    fn invalidate_filter(&mut self) -> Result<(), String> {
        self.filter = None;
        self.filter_valid = false;
        self.clear_visible_index()
            .map_err(|error| error.to_string())
    }

    fn clear_visible_index(&mut self) -> io::Result<()> {
        self.flush()?;
        let writer = self
            .visible_index_writer
            .as_mut()
            .expect("visible index writer");
        writer.get_ref().set_len(0)?;
        writer.seek(SeekFrom::Start(0))?;
        self.visible_count = 0;
        self.clear_find_index()
    }

    fn filter_matches(&self, line: &str) -> bool {
        self.filter_valid
            && self
                .filter
                .as_ref()
                .is_none_or(|filter| filter.is_match(line))
    }

    fn find_matches(&self, line: &str) -> bool {
        self.find.as_ref().is_some_and(|find| find.is_match(line))
    }

    fn rebuild_find_index(&mut self) -> io::Result<()> {
        self.flush()?;
        self.clear_find_index()?;
        if self.find_query.is_empty() {
            return Ok(());
        }
        let find = self.find.clone();
        let visible_reader = self
            .visible_index_reader
            .as_mut()
            .expect("visible index reader");
        visible_reader.seek(SeekFrom::Start(0))?;
        let mut visible_reader = BufReader::with_capacity(64 * 1024, visible_reader);
        let raw_index_reader = self.raw_index_reader.as_mut().expect("raw index reader");
        raw_index_reader.seek(SeekFrom::Start(0))?;
        let mut raw_index_reader = BufReader::with_capacity(64 * 1024, raw_index_reader);
        let data_reader = self.data_reader.as_mut().expect("raw data reader");
        data_reader.seek(SeekFrom::Start(0))?;
        let mut data_reader = BufReader::with_capacity(64 * 1024, data_reader);
        let mut next_raw_index = 0;
        let mut next_data_offset = 0;
        for visible_index in 0..self.visible_count {
            let raw_index = read_next_u64(&mut visible_reader)?;
            raw_index_reader.seek_relative(((raw_index - next_raw_index) * 8) as i64)?;
            let data_offset = read_next_u64(&mut raw_index_reader)?;
            next_raw_index = raw_index + 1;
            data_reader.seek_relative((data_offset - next_data_offset) as i64)?;
            let line = read_next_record(&mut data_reader)?;
            next_data_offset = data_offset + 8 + line.len() as u64;
            if find.as_ref().is_some_and(|find| find.is_match(&line)) {
                append_u64(
                    self.find_index_writer.as_mut().expect("find index writer"),
                    visible_index,
                )?;
                self.find_count += 1;
            }
        }
        self.last_rebuild_work.visible_records_scanned = self.visible_count;
        self.find_index_writer
            .as_mut()
            .expect("find index writer")
            .flush()
    }

    fn clear_find_index(&mut self) -> io::Result<()> {
        let writer = self.find_index_writer.as_mut().expect("find index writer");
        writer.flush()?;
        writer.get_ref().set_len(0)?;
        writer.seek(SeekFrom::Start(0))?;
        self.find_count = 0;
        Ok(())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.data_writer
            .as_mut()
            .expect("raw data writer")
            .flush()?;
        self.raw_index_writer
            .as_mut()
            .expect("raw index writer")
            .flush()?;
        self.visible_index_writer
            .as_mut()
            .expect("visible index writer")
            .flush()?;
        self.find_index_writer
            .as_mut()
            .expect("find index writer")
            .flush()
    }
}

impl Drop for SessionLogStore {
    fn drop(&mut self) {
        drop(self.data_writer.take());
        drop(self.data_reader.take());
        drop(self.raw_index_writer.take());
        drop(self.raw_index_reader.take());
        drop(self.visible_index_writer.take());
        drop(self.visible_index_reader.take());
        drop(self.find_index_writer.take());
        drop(self.find_index_reader.take());
        let _ = fs::remove_dir_all(&self.directory);
    }
}

fn create_session_directory() -> io::Result<PathBuf> {
    for _ in 0..100 {
        let id = NEXT_SESSION_ID.fetch_add(1, Ordering::Relaxed);
        let directory = std::env::temp_dir().join(format!("arklog-{}-{id}", std::process::id()));
        match fs::create_dir(&directory) {
            Ok(()) => return Ok(directory),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "could not allocate a unique ArkLog session directory",
    ))
}

fn create_file_pair(path: PathBuf) -> io::Result<(BufWriter<File>, File)> {
    let writer = OpenOptions::new()
        .create_new(true)
        .read(true)
        .write(true)
        .open(&path)?;
    let reader = OpenOptions::new().read(true).open(path)?;
    Ok((BufWriter::new(writer), reader))
}

fn append_u64(file: &mut BufWriter<File>, value: u64) -> io::Result<()> {
    file.write_all(&value.to_le_bytes())
}

fn read_next_u64(reader: &mut impl Read) -> io::Result<u64> {
    let mut bytes = [0; 8];
    reader.read_exact(&mut bytes)?;
    Ok(u64::from_le_bytes(bytes))
}

fn read_u64(file: &mut File, offset: u64) -> io::Result<u64> {
    let mut bytes = [0; 8];
    file.seek(SeekFrom::Start(offset))?;
    file.read_exact(&mut bytes)?;
    Ok(u64::from_le_bytes(bytes))
}

fn read_record(file: &mut File, offset: u64) -> io::Result<String> {
    file.seek(SeekFrom::Start(offset))?;
    read_next_record(file)
}

fn read_next_record(file: &mut impl Read) -> io::Result<String> {
    let mut length = [0; 8];
    file.read_exact(&mut length)?;
    let mut bytes = vec![0; u64::from_le_bytes(length) as usize];
    file.read_exact(&mut bytes)?;
    String::from_utf8(bytes).map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
}
