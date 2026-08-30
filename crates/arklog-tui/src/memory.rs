use std::io;
use std::time::Instant;

use crate::SessionLogStore;

pub const MEMORY_BUDGET_BYTES: u64 = 50 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MemoryProbe {
    pub raw_count: u64,
    pub visible_count: u64,
    pub find_count: u64,
    pub retained_heap_bytes: usize,
    pub rss_bytes: u64,
    pub append_millis: u128,
    pub filter_millis: u128,
    pub find_millis: u128,
}

pub fn run_memory_probe(line_count: u64) -> io::Result<MemoryProbe> {
    let mut store = SessionLogStore::new()?;
    let append_started = Instant::now();
    for index in 0..line_count {
        store.append_lines([format!(
            "08-30 20:10:01.001 120 121 I ArkUI/Render frame committed seq={index:010} payload=abcdefghijklmnopqrstuvwxyz0123456789"
        )])?;
    }
    let append_millis = append_started.elapsed().as_millis();
    let filter_started = Instant::now();
    store
        .set_filter(r"seq=\d*[02468] payload=")
        .map_err(io::Error::other)?;
    let filter_millis = filter_started.elapsed().as_millis();
    let find_started = Instant::now();
    store.set_find("payload=")?;
    let find_millis = find_started.elapsed().as_millis();
    let start = store.visible_count().saturating_sub(32);
    let latest = store.visible_window(start, 32)?;
    if latest.len() != store.visible_count().min(32) as usize {
        return Err(io::Error::other(
            "memory probe could not read the latest window",
        ));
    }
    Ok(MemoryProbe {
        raw_count: store.raw_count(),
        visible_count: store.visible_count(),
        find_count: store.find_count(),
        retained_heap_bytes: store.retained_heap_bytes(),
        rss_bytes: process_peak_rss_bytes()?,
        append_millis,
        filter_millis,
        find_millis,
    })
}

#[cfg(target_os = "macos")]
fn process_peak_rss_bytes() -> io::Result<u64> {
    let mut usage = std::mem::MaybeUninit::<libc::rusage>::uninit();
    let status = unsafe { libc::getrusage(libc::RUSAGE_SELF, usage.as_mut_ptr()) };
    if status != 0 {
        return Err(io::Error::last_os_error());
    }
    let usage = unsafe { usage.assume_init() };
    Ok(usage.ru_maxrss as u64)
}

#[cfg(all(unix, not(target_os = "macos")))]
fn process_peak_rss_bytes() -> io::Result<u64> {
    let mut usage = std::mem::MaybeUninit::<libc::rusage>::uninit();
    let status = unsafe { libc::getrusage(libc::RUSAGE_SELF, usage.as_mut_ptr()) };
    if status != 0 {
        return Err(io::Error::last_os_error());
    }
    let usage = unsafe { usage.assume_init() };
    Ok((usage.ru_maxrss as u64) * 1024)
}

#[cfg(windows)]
fn process_peak_rss_bytes() -> io::Result<u64> {
    #[repr(C)]
    struct ProcessMemoryCounters {
        cb: u32,
        page_fault_count: u32,
        peak_working_set_size: usize,
        working_set_size: usize,
        quota_peak_paged_pool_usage: usize,
        quota_paged_pool_usage: usize,
        quota_peak_non_paged_pool_usage: usize,
        quota_non_paged_pool_usage: usize,
        pagefile_usage: usize,
        peak_pagefile_usage: usize,
    }
    #[link(name = "kernel32")]
    extern "system" {
        fn GetCurrentProcess() -> *mut std::ffi::c_void;
    }
    #[link(name = "psapi")]
    extern "system" {
        fn GetProcessMemoryInfo(
            process: *mut std::ffi::c_void,
            counters: *mut ProcessMemoryCounters,
            size: u32,
        ) -> i32;
    }
    let mut counters = ProcessMemoryCounters {
        cb: std::mem::size_of::<ProcessMemoryCounters>() as u32,
        page_fault_count: 0,
        peak_working_set_size: 0,
        working_set_size: 0,
        quota_peak_paged_pool_usage: 0,
        quota_paged_pool_usage: 0,
        quota_peak_non_paged_pool_usage: 0,
        quota_non_paged_pool_usage: 0,
        pagefile_usage: 0,
        peak_pagefile_usage: 0,
    };
    let status = unsafe { GetProcessMemoryInfo(GetCurrentProcess(), &mut counters, counters.cb) };
    if status == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(counters.peak_working_set_size as u64)
    }
}
