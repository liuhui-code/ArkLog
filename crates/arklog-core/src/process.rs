use std::io;
use std::process::Command;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub(crate) use windows::ManagedChild;
#[cfg(not(windows))]
pub(crate) type ManagedChild = std::process::Child;

pub(crate) fn spawn_command(command: &mut Command) -> io::Result<ManagedChild> {
    #[cfg(windows)]
    {
        windows::spawn(command)
    }
    #[cfg(not(windows))]
    {
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            command.process_group(0);
        }
        command.spawn()
    }
}

pub(crate) fn terminate_process_tree(child: &mut ManagedChild) -> io::Result<()> {
    #[cfg(windows)]
    {
        child.terminate_tree()
    }
    #[cfg(not(windows))]
    {
        #[cfg(unix)]
        {
            let process_group = -(child.id() as i32);
            if unsafe { libc::kill(process_group, libc::SIGKILL) } == 0 {
                return Ok(());
            }
        }
        child.kill()
    }
}
