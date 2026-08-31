use std::io::Read;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug)]
pub struct CommandPolicy {
    timeout: Duration,
    max_output_bytes: usize,
}

impl CommandPolicy {
    pub const fn new(timeout: Duration, max_output_bytes: usize) -> Self {
        Self {
            timeout,
            max_output_bytes,
        }
    }
}

pub struct CommandOutput {
    pub success: bool,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

pub trait CommandRunner: Send + Sync {
    fn output(&self, program: &str, args: &[String]) -> Result<CommandOutput, String>;

    fn output_with_policy(
        &self,
        program: &str,
        args: &[String],
        policy: CommandPolicy,
    ) -> Result<CommandOutput, String> {
        let output = self.output(program, args)?;
        let bytes = output.stdout.len().saturating_add(output.stderr.len());
        if bytes > policy.max_output_bytes {
            return Err(format!(
                "Command output exceeded {} bytes",
                policy.max_output_bytes
            ));
        }
        Ok(output)
    }
}

pub struct SystemCommandRunner;

impl CommandRunner for SystemCommandRunner {
    fn output(&self, program: &str, args: &[String]) -> Result<CommandOutput, String> {
        self.output_with_policy(
            program,
            args,
            CommandPolicy::new(Duration::from_secs(5), 8 * 1024 * 1024),
        )
    }

    fn output_with_policy(
        &self,
        program: &str,
        args: &[String],
        policy: CommandPolicy,
    ) -> Result<CommandOutput, String> {
        run_command(program, args, policy)
    }
}

fn run_command(
    program: &str,
    args: &[String],
    policy: CommandPolicy,
) -> Result<CommandOutput, String> {
    let mut command = Command::new(program);
    configure_hidden_command(&mut command);
    let mut child = command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("Failed to run {program}: {error}"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| format!("Failed to capture {program} stdout"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| format!("Failed to capture {program} stderr"))?;
    let used = Arc::new(AtomicUsize::new(0));
    let stdout_reader = spawn_bounded_reader(stdout, Arc::clone(&used), policy.max_output_bytes);
    let stderr_reader = spawn_bounded_reader(stderr, Arc::clone(&used), policy.max_output_bytes);
    let deadline = Instant::now() + policy.timeout;
    let status = loop {
        if let Some(status) = child
            .try_wait()
            .map_err(|error| format!("Failed to inspect {program}: {error}"))?
        {
            break status;
        }
        if used.load(Ordering::Relaxed) > policy.max_output_bytes {
            terminate_child(program, &mut child, stdout_reader, stderr_reader)?;
            return Err(format!(
                "Command output exceeded {} bytes",
                policy.max_output_bytes
            ));
        }
        if Instant::now() >= deadline {
            terminate_child(program, &mut child, stdout_reader, stderr_reader)?;
            return Err(format!(
                "Command timed out after {} ms: {program}",
                policy.timeout.as_millis()
            ));
        }
        thread::sleep(Duration::from_millis(10));
    };
    let stdout = join_reader(stdout_reader)?;
    let stderr = join_reader(stderr_reader)?;
    Ok(CommandOutput {
        success: status.success(),
        stdout,
        stderr,
    })
}

fn terminate_child(
    program: &str,
    child: &mut std::process::Child,
    stdout_reader: thread::JoinHandle<Result<Vec<u8>, String>>,
    stderr_reader: thread::JoinHandle<Result<Vec<u8>, String>>,
) -> Result<(), String> {
    terminate_process_tree(child)
        .map_err(|error| format!("Failed to terminate {program}: {error}"))?;
    child
        .wait()
        .map_err(|error| format!("Failed to reap {program}: {error}"))?;
    let _ = stdout_reader.join();
    let _ = stderr_reader.join();
    Ok(())
}

fn spawn_bounded_reader<R: Read + Send + 'static>(
    mut reader: R,
    used: Arc<AtomicUsize>,
    limit: usize,
) -> thread::JoinHandle<Result<Vec<u8>, String>> {
    thread::spawn(move || {
        let mut output = Vec::new();
        let mut buffer = [0_u8; 8 * 1024];
        loop {
            let count = reader
                .read(&mut buffer)
                .map_err(|error| format!("Failed to read command output: {error}"))?;
            if count == 0 {
                return Ok(output);
            }
            let previous = used.fetch_add(count, Ordering::Relaxed);
            let remaining = limit.saturating_sub(previous);
            output.extend_from_slice(&buffer[..count.min(remaining)]);
            if count > remaining {
                return Err(format!("Command output exceeded {limit} bytes"));
            }
        }
    })
}

fn join_reader(reader: thread::JoinHandle<Result<Vec<u8>, String>>) -> Result<Vec<u8>, String> {
    reader
        .join()
        .map_err(|_| "Command output reader panicked".to_string())?
}

pub(crate) fn configure_hidden_command(command: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    #[cfg(not(any(unix, windows)))]
    let _ = command;
}

pub(crate) fn terminate_process_tree(child: &mut std::process::Child) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        let process_group = -(child.id() as i32);
        let status = unsafe { libc::kill(process_group, libc::SIGKILL) };
        if status == 0 {
            return Ok(());
        }
    }
    child.kill()
}
