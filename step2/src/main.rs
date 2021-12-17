use nix::{
    sys::wait::waitpid,
    unistd::{execvp, fork, ForkResult},
};
use std::{
    ffi::CString,
    io::{stdin, stdout, Write},
};

#[derive(Debug)]
enum Action {
    SimpleCommand(Vec<String>),
}

fn main() {
    shell_loop()
}

fn shell_loop() {
    while let Some(line) = shell_read_line() {
        let action = match shell_parse_line(&line) {
            None => continue,
            Some(action) => action,
        };

        match action {
            Action::SimpleCommand(command) => shell_exec_simple_command(command),
            // _ => unimplemented![],
        }
    }
}

fn shell_read_line() -> Option<String> {
    print!("> ");
    stdout().flush().unwrap(); // バッファリング対策

    let mut result = String::new();
    match stdin().read_line(&mut result) {
        Ok(size) => {
            if size == 0 {
                None
            } else {
                // 改行を削除
                let result = result.trim_end();
                Some(result.to_string())
            }
        }
        Err(e) => {
            eprintln!("{}", e);
            None
        }
    }
}

fn shell_parse_line(line: &str) -> Option<Action> {
    match line.is_empty() {
        true => None,
        false => {
            // lineを空白で分割
            let commands = line.split(' ').map(|s| s.to_string()).collect::<Vec<_>>();
            Some(Action::SimpleCommand(commands))
        }
    }
}

fn shell_exec_simple_command(command: Vec<String>) {
    match unsafe { fork() } {
        Ok(ForkResult::Parent { child, .. }) => {
            waitpid(child, None).unwrap();
        }
        Ok(ForkResult::Child) => {
            let args = command
                .into_iter()
                // Nullは含まれていないと信じてunwrapする
                .map(|c| CString::new(c).unwrap())
                .collect::<Vec<_>>();

            execvp(&args[0], &args).unwrap();
        }
        Err(e) => {
            eprintln!("fork error: {}", e);
        }
    }
}
