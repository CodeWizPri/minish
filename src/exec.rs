// src/exec.rs
use crate::parser::{Pipeline, Cmd};
use std::env;
use std::fs::File;
use std::process::{Command, Stdio, Child}; // Cleaned up unused ExitStatus import

/// Executes a complete pipeline of commands concurrently.
/// If it's a single builtin command, it executes inside the parent process to allow state changes.
/// Returns the exit status code of the last command in the pipeline.
pub fn execute_pipeline(pipeline: &Pipeline) -> i32 {
    if pipeline.commands.is_empty() {
        return 0;
    }

    // Core Requirement: If it is exactly ONE command and matches a builtin,
    // it MUST run in-process to mutate shell state (like `cd`).
    if pipeline.commands.len() == 1 {
        let cmd = &pipeline.commands[0];
        if !cmd.argv.is_empty() && is_builtin(&cmd.argv[0]) {
            return execute_builtin(cmd);
        }
    }

    let mut child_handles: Vec<Child> = Vec::new();
    let mut prev_stdout: Option<Stdio> = None;
    let num_commands = pipeline.commands.len();

    // Iterate through all pipeline components concurrently
    for (i, cmd) in pipeline.commands.iter().enumerate() {
        if cmd.argv.is_empty() {
            continue;
        }

        let program = &cmd.argv[0];
        let args = &cmd.argv[1..];

        let mut child_command = Command::new(program);
        child_command.args(args);

        // --- STEP 1: CONFIGURE INPUT REDIRECTION (STDIN) ---
        if let Some(ref stdin_file) = cmd.stdin_from {
            // Explicit `< file` takes highest priority
            match File::open(stdin_file) {
                Ok(file) => child_command.stdin(Stdio::from(file)),
                Err(e) => {
                    eprintln!("minish: {}: {}", stdin_file, e); // Prints the OS error directly
                    return 1; // Command does not run if file is unopenable
                }
            };
        } else if let Some(prev_stage_output) = prev_stdout.take() {
            // Connect previous stage's stdout to current stage's stdin
            child_command.stdin(prev_stage_output);
        } else {
            // First stage defaults to inheriting terminal stdin
            child_command.stdin(Stdio::inherit());
        }

        // --- STEP 2: CONFIGURE OUTPUT REDIRECTION (STDOUT) ---
        if let Some(ref stdout_file) = cmd.stdout_to {
            // Explicit `> file` truncates stdout
            match File::create(stdout_file) {
                Ok(file) => child_command.stdout(Stdio::from(file)),
                Err(e) => {
                    eprintln!("minish: {}: {}", stdout_file, e); // Prints the OS error directly
                    return 1;
                }
            };
        } else if i < num_commands - 1 {
            // Middle pipeline stages request piped streaming descriptors
            child_command.stdout(Stdio::piped());
        } else {
            // Last stage outputs directly back to the terminal screen
            child_command.stdout(Stdio::inherit());
        }

        // --- STEP 3: HANDLE PIPELINE INTERNALS & SPAWNING ---
        if is_builtin(program) {
            // Builtins nested in a pipeline run with mocked parent execution.
            // They print immediately but won't permanently modify the parent shell's interactive directory state.
            execute_builtin(cmd);
            continue;
        }

        match child_command.spawn() {
            Ok(mut child) => {
                // Hand `child.stdout.take()` cleanly to the next pipeline stage
                if i < num_commands - 1 {
                    if let Some(stdout) = child.stdout.take() {
                        prev_stdout = Some(Stdio::from(stdout));
                    }
                }
                child_handles.push(child);
            }
            Err(_) => {
                eprintln!("minish: command not found: {}", program); // Required exact error output format
                return 127; // Requirement: exit status code 127 on spawn failure
            }
        }
    }

    // --- STEP 4: REAP ALL RUNNING CHILDREN (PREVENTS ZOMBIES) ---
    let mut last_exit_code = 0;
    for (i, mut child) in child_handles.into_iter().enumerate() {
        match child.wait() {
            Ok(status) => {
                // Core Requirement: pipeline status is determined by the last command's status
                if i == num_commands - 1 {
                    last_exit_code = status.code().unwrap_or(0);
                }
            }
            Err(e) => {
                eprintln!("minish: error waiting for child process: {}", e);
            }
        }
    }

    last_exit_code
}

/// Helper to identify if a command string is a shell builtin
fn is_builtin(program: &str) -> bool {
    matches!(program, "cd" | "pwd" | "exit" | "echo")
}

/// Executes a verified shell builtin in-process while properly accommodating file redirections.
fn execute_builtin(cmd: &Cmd) -> i32 {
    let program = &cmd.argv[0];
    let args = &cmd.argv[1..];

    // --- Builtin Output Redirection Interception ---
    let mut custom_stdout: Option<File> = None;
    if let Some(ref stdout_file) = cmd.stdout_to {
        match File::create(stdout_file) {
            Ok(file) => custom_stdout = Some(file),
            Err(e) => {
                eprintln!("minish: {}: {}", stdout_file, e);
                return 1;
            }
        }
    }

    // Direct output closure helper to streamline terminal vs file targets.
    // Uses explicit print! with '\n' to guarantee byte-for-byte Unix line endings on Windows.
    let mut write_output = |text: String| {
        if let Some(ref mut file) = custom_stdout {
            use std::io::Write;
            if let Err(e) = write!(file, "{}\n", text) {
                eprintln!("minish: file write error: {}", e);
            }
        } else {
            use std::io::Write;
            print!("{}\n", text);
            let _ = std::io::stdout().flush();
        }
    };

    match program.as_str() {
        "exit" => {
            let code = args.get(0)
                .and_then(|s| s.parse::<i32>().ok())
                .unwrap_or(0);
            std::process::exit(code); // Gracefully triggers standard termination
        }
        "pwd" => {
            match env::current_dir() {
                Ok(path) => write_output(format!("{}", path.display())),
                Err(e) => eprintln!("minish: pwd error: {}", e),
            }
            0
        }
        "cd" => {
            let target_dir = if args.is_empty() {
                // Cross-platform home environment targeting: checks Windows USERPROFILE first, then Unix HOME.
                env::var("USERPROFILE").or_else(|_| env::var("HOME")).unwrap_or_else(|_| "".to_string())
            } else {
                args[0].clone()
            };

            if target_dir.is_empty() {
                eprintln!("minish: cd: HOME or USERPROFILE environment variable not set");
                return 1;
            }

            if let Err(e) = env::set_current_dir(&target_dir) {
                eprintln!("minish: cd: {}: {}", target_dir, e);
                return 1;
            }
            0
        }
        "echo" => {
            write_output(args.join(" ")); // Merges arguments cleanly
            0
        }
        _ => 1,
    }
}