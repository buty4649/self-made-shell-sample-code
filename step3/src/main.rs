use nix::{
    errno::Errno,
    sys::{
        signal::{sigaction, SaFlags, SigAction, SigHandler, SigSet, Signal},
        wait::waitpid,
    },
    unistd::{close, execvp, fork, getpgrp, pipe, read, setpgid, tcsetpgrp, ForkResult},
};
use std::{
    ffi::CString,
    io::{stdin, stdout, Write},
    process::exit,
};

#[derive(Debug)]
enum Action {
    SimpleCommand(Vec<String>),
}

fn main() {
    shell_loop()
}

fn shell_loop() {
    ignore_tty_signals();

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
            if size == 0 || result.is_empty() {
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
    let (pipe_read, pipe_write) = pipe().unwrap();

    match unsafe { fork() } {
        Ok(ForkResult::Parent { child, .. }) => {
            // 子プロセスをプロセスグループリーダーにする
            setpgid(child, child).unwrap();

            // 子プロセスのプロセスグループをフォアグラウンドプロセスに設定する
            tcsetpgrp(0, child).unwrap();

            // 子プロセスとの同期を終了する
            close(pipe_read).unwrap();
            close(pipe_write).unwrap();

            waitpid(child, None).ok();

            // 自分のプロセスグループをフォアグラウンドプロセスに戻す
            tcsetpgrp(0, getpgrp()).unwrap();
        }
        Ok(ForkResult::Child) => {
            // シグナルアクションは親プロセスから継承されるためデフォルトに戻す
            restore_tty_signals();

            // 不要なパイプは閉じておく
            close(pipe_write).unwrap();

            // 親プロセスの処理が終わるまで待機する
            loop {
                let mut buf = [0];
                match read(pipe_read, &mut buf) {
                    // シグナルによる割り込みを無視
                    Err(e) if e == Errno::EINTR => (),
                    _ => break,
                }
            }
            close(pipe_read).unwrap();

            let args = command
                .into_iter()
                // Nullは含まれていないと信じてunwrapする
                .map(|c| CString::new(c).unwrap())
                .collect::<Vec<_>>();

            match execvp(&args[0], &args) {
                Ok(_) => (),
                Err(e) if e == Errno::ENOENT => (),
                Err(e) => {
                    eprintln!("Error: {}", e);
                    exit(1);
                }
            };
            exit(0);
        }
        Err(e) => {
            eprintln!("fork error: {}", e);
        }
    }
}

fn ignore_tty_signals() {
    let sa = SigAction::new(SigHandler::SigIgn, SaFlags::empty(), SigSet::empty());
    unsafe {
        sigaction(Signal::SIGTSTP, &sa).unwrap();
        sigaction(Signal::SIGTTIN, &sa).unwrap();
        sigaction(Signal::SIGTTOU, &sa).unwrap();
    }
}

fn restore_tty_signals() {
    let sa = SigAction::new(SigHandler::SigDfl, SaFlags::empty(), SigSet::empty());
    unsafe {
        sigaction(Signal::SIGTSTP, &sa).unwrap();
        sigaction(Signal::SIGTTIN, &sa).unwrap();
        sigaction(Signal::SIGTTOU, &sa).unwrap();
    }
}
