use std::ffi::c_void;
use std::io;
use std::ops::{Deref, DerefMut};
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle, RawHandle};
use std::os::windows::process::CommandExt;
use std::process::{Child, Command};

pub(crate) struct ManagedChild {
    child: Child,
    job: OwnedHandle,
}

impl ManagedChild {
    pub(super) fn terminate_tree(&mut self) -> io::Result<()> {
        check(unsafe { TerminateJobObject(self.job.as_raw_handle(), 1) })
    }
}

impl Deref for ManagedChild {
    type Target = Child;

    fn deref(&self) -> &Child {
        &self.child
    }
}

impl DerefMut for ManagedChild {
    fn deref_mut(&mut self) -> &mut Child {
        &mut self.child
    }
}

pub(super) fn spawn(command: &mut Command) -> io::Result<ManagedChild> {
    let job = own(unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) })?;
    // Assign before the primary thread can create children or exit. The job
    // remains usable for cleanup even after that original process has exited.
    const CREATE_SUSPENDED: u32 = 0x0000_0004;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let mut child = command
        .creation_flags(CREATE_NO_WINDOW | CREATE_SUSPENDED)
        .spawn()?;
    let assigned =
        check(unsafe { AssignProcessToJobObject(job.as_raw_handle(), child.as_raw_handle()) });
    if let Err(error) = assigned.and_then(|()| resume_primary_thread(child.id())) {
        let _ = child.kill();
        let _ = child.wait();
        return Err(error);
    }
    // No KILL_ON_JOB_CLOSE: a successful discovery command may have started
    // the shared HDC server. Only explicit stop/timeout/reap terminates the job.
    Ok(ManagedChild { child, job })
}

fn resume_primary_thread(process_id: u32) -> io::Result<()> {
    const TH32CS_SNAPTHREAD: u32 = 0x0000_0004;
    const THREAD_SUSPEND_RESUME: u32 = 0x0000_0002;
    let snapshot = own(unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0) })?;
    let mut entry = ThreadEntry {
        size: std::mem::size_of::<ThreadEntry>() as u32,
        ..Default::default()
    };
    let mut found = unsafe { Thread32First(snapshot.as_raw_handle(), &mut entry) };
    while found != 0 {
        if entry.owner_process_id == process_id {
            let thread = own(unsafe { OpenThread(THREAD_SUSPEND_RESUME, 0, entry.thread_id) })?;
            if unsafe { ResumeThread(thread.as_raw_handle()) } == u32::MAX {
                return Err(io::Error::last_os_error());
            }
            return Ok(());
        }
        entry.size = std::mem::size_of::<ThreadEntry>() as u32;
        found = unsafe { Thread32Next(snapshot.as_raw_handle(), &mut entry) };
    }
    Err(io::Error::new(
        io::ErrorKind::NotFound,
        "HDC primary thread was not found",
    ))
}

fn own(handle: RawHandle) -> io::Result<OwnedHandle> {
    if handle.is_null() || handle == -1_isize as RawHandle {
        Err(io::Error::last_os_error())
    } else {
        // Every caller transfers a newly created, uniquely owned kernel handle.
        Ok(unsafe { OwnedHandle::from_raw_handle(handle) })
    }
}

fn check(result: i32) -> io::Result<()> {
    if result == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[repr(C)]
#[derive(Default)]
struct ThreadEntry {
    size: u32,
    usage: u32,
    thread_id: u32,
    owner_process_id: u32,
    base_priority: i32,
    delta_priority: i32,
    flags: u32,
}

#[link(name = "Kernel32")]
extern "system" {
    fn CreateJobObjectW(attributes: *const c_void, name: *const u16) -> RawHandle;
    fn AssignProcessToJobObject(job: RawHandle, process: RawHandle) -> i32;
    fn TerminateJobObject(job: RawHandle, exit_code: u32) -> i32;
    fn CreateToolhelp32Snapshot(flags: u32, process_id: u32) -> RawHandle;
    fn Thread32First(snapshot: RawHandle, entry: *mut ThreadEntry) -> i32;
    fn Thread32Next(snapshot: RawHandle, entry: *mut ThreadEntry) -> i32;
    fn OpenThread(access: u32, inherit: i32, thread_id: u32) -> RawHandle;
    fn ResumeThread(thread: RawHandle) -> u32;
}
