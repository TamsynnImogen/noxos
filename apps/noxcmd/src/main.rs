#[cfg(feature = "qml")]
mod qml_app {
    use qmetaobject::listmodel::QAbstractListModel;
    use qmetaobject::prelude::*;
    use std::collections::HashMap;
    use std::env;
    use std::path::{Path, PathBuf};
    use std::process::{Command, ExitCode};
    use std::time::{Instant, SystemTime, UNIX_EPOCH};

    const ROLE_COMMAND: i32 = qmetaobject::USER_ROLE;
    const ROLE_CWD: i32 = qmetaobject::USER_ROLE + 1;
    const ROLE_OUTPUT: i32 = qmetaobject::USER_ROLE + 2;
    const ROLE_EXIT_CODE: i32 = qmetaobject::USER_ROLE + 3;
    const ROLE_DURATION_MS: i32 = qmetaobject::USER_ROLE + 4;
    const ROLE_STARTED_AT: i32 = qmetaobject::USER_ROLE + 5;
    const ROLE_SUCCESS: i32 = qmetaobject::USER_ROLE + 6;

    #[derive(Clone, Default)]
    struct CommandBlock {
        command: QString,
        cwd: QString,
        output: QString,
        exit_code: i32,
        duration_ms: u64,
        started_at: u64,
        success: bool,
    }

    #[derive(QObject, Default)]
    struct CommandBlocksModel {
        #[qt_base_class = "QAbstractListModel"]
        base: qmetaobject::QObjectCppWrapper,
        blocks: Vec<CommandBlock>,
        current_directory: qt_property!(QString; NOTIFY current_directory_changed),
        current_directory_changed: qt_signal!(),
        error_message: qt_property!(QString; NOTIFY error_message_changed),
        error_message_changed: qt_signal!(),
        run_command: qt_method!(
            fn run_command(&mut self, command: QString) {
                let command_text = command.to_string();
                let trimmed = command_text.trim();
                if trimmed.is_empty() {
                    return;
                }

                let cwd = PathBuf::from(self.current_directory.to_string());
                if let Some(next_dir) = resolve_cd_command(trimmed, &cwd) {
                    self.push_cd_block(trimmed, &cwd, &next_dir);
                    self.set_current_directory_path(next_dir);
                    return;
                }

                let block = execute_command_block(trimmed, &cwd);
                self.begin_insert_rows(self.blocks.len() as i32, self.blocks.len() as i32);
                self.blocks.push(block);
                self.end_insert_rows();
            }
        ),
        set_current_directory: qt_method!(
            fn set_current_directory(&mut self, directory: QString) {
                self.set_current_directory_path(PathBuf::from(directory.to_string()));
            }
        ),
        clear_blocks: qt_method!(
            fn clear_blocks(&mut self) {
                self.begin_reset_model();
                self.blocks.clear();
                self.end_reset_model();
            }
        ),
    }

    impl CommandBlocksModel {
        fn set_current_directory_path(&mut self, directory: PathBuf) {
            let resolved = directory.canonicalize().unwrap_or(directory);
            if !resolved.is_dir() {
                self.error_message =
                    QString::from(format!("Not a directory: {}", resolved.display()));
                self.error_message_changed();
                return;
            }

            self.current_directory = QString::from(resolved.display().to_string());
            self.current_directory_changed();
            self.error_message = QString::default();
            self.error_message_changed();
        }

        fn push_cd_block(&mut self, command: &str, previous_cwd: &Path, next_dir: &Path) {
            let output = format!("{}", next_dir.display());
            let block = CommandBlock {
                command: QString::from(command),
                cwd: QString::from(previous_cwd.display().to_string()),
                output: QString::from(output),
                exit_code: 0,
                duration_ms: 0,
                started_at: unix_millis(),
                success: true,
            };

            self.begin_insert_rows(self.blocks.len() as i32, self.blocks.len() as i32);
            self.blocks.push(block);
            self.end_insert_rows();
        }
    }

    impl QAbstractListModel for CommandBlocksModel {
        fn row_count(&self) -> i32 {
            self.blocks.len() as i32
        }

        fn data(&self, index: QModelIndex, role: i32) -> QVariant {
            let row = index.row();
            if row < 0 || row as usize >= self.blocks.len() {
                return QVariant::default();
            }

            let block = &self.blocks[row as usize];
            match role {
                ROLE_COMMAND => block.command.clone().into(),
                ROLE_CWD => block.cwd.clone().into(),
                ROLE_OUTPUT => block.output.clone().into(),
                ROLE_EXIT_CODE => block.exit_code.into(),
                ROLE_DURATION_MS => (block.duration_ms as f64).into(),
                ROLE_STARTED_AT => (block.started_at as f64).into(),
                ROLE_SUCCESS => block.success.into(),
                _ => QVariant::default(),
            }
        }

        fn role_names(&self) -> HashMap<i32, QByteArray> {
            HashMap::from([
                (ROLE_COMMAND, QByteArray::from("command")),
                (ROLE_CWD, QByteArray::from("cwd")),
                (ROLE_OUTPUT, QByteArray::from("output")),
                (ROLE_EXIT_CODE, QByteArray::from("exitCode")),
                (ROLE_DURATION_MS, QByteArray::from("durationMs")),
                (ROLE_STARTED_AT, QByteArray::from("startedAt")),
                (ROLE_SUCCESS, QByteArray::from("success")),
            ])
        }
    }

    fn execute_command_block(command: &str, cwd: &Path) -> CommandBlock {
        let started_at = unix_millis();
        let start = Instant::now();
        let result = Command::new("bash")
            .arg("-lc")
            .arg(command)
            .current_dir(cwd)
            .output();
        let duration_ms = start.elapsed().as_millis() as u64;

        match result {
            Ok(output) => {
                let mut text = String::new();
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                if !stdout.trim_end().is_empty() {
                    text.push_str(stdout.trim_end());
                }
                if !stderr.trim_end().is_empty() {
                    if !text.is_empty() {
                        text.push('\n');
                    }
                    text.push_str(stderr.trim_end());
                }

                CommandBlock {
                    command: QString::from(command),
                    cwd: QString::from(cwd.display().to_string()),
                    output: QString::from(text),
                    exit_code: output.status.code().unwrap_or(-1),
                    duration_ms,
                    started_at,
                    success: output.status.success(),
                }
            }
            Err(error) => CommandBlock {
                command: QString::from(command),
                cwd: QString::from(cwd.display().to_string()),
                output: QString::from(format!("Failed to run command: {error}")),
                exit_code: -1,
                duration_ms,
                started_at,
                success: false,
            },
        }
    }

    fn resolve_cd_command(command: &str, cwd: &Path) -> Option<PathBuf> {
        let target = if command == "cd" {
            env::var_os("HOME").map(PathBuf::from)?
        } else if let Some(rest) = command.strip_prefix("cd ") {
            let trimmed = rest.trim();
            if trimmed.contains("&&") || trimmed.contains(';') || trimmed.contains('|') {
                return None;
            }
            expand_shellish_path(trimmed, cwd)
        } else {
            return None;
        };

        target.canonicalize().ok().filter(|path| path.is_dir())
    }

    fn expand_shellish_path(value: &str, cwd: &Path) -> PathBuf {
        let unquoted = value.trim_matches('"').trim_matches('\'');
        if unquoted == "~" {
            return env::var_os("HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|| cwd.to_path_buf());
        }
        if let Some(suffix) = unquoted.strip_prefix("~/") {
            return env::var_os("HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|| cwd.to_path_buf())
                .join(suffix);
        }

        let path = PathBuf::from(unquoted);
        if path.is_absolute() {
            path
        } else {
            cwd.join(path)
        }
    }

    fn unix_millis() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|value| value.as_millis() as u64)
            .unwrap_or(0)
    }

    fn initial_directory() -> PathBuf {
        env::args_os()
            .nth(1)
            .map(PathBuf::from)
            .unwrap_or_else(|| env::current_dir().expect("failed to determine current directory"))
    }

    fn qml_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("qml/Main.qml")
    }

    pub fn run() -> ExitCode {
        let mut engine = QmlEngine::new();
        let model = QObjectBox::new(CommandBlocksModel::default());

        {
            let pinned_model = model.pinned();
            let mut pinned = pinned_model.borrow_mut();
            pinned.set_current_directory_path(initial_directory());
        }

        engine.set_object_property(QString::from("commandBlocksModel"), model.pinned());
        engine.load_file(QString::from(qml_path().display().to_string()));
        engine.exec();
        ExitCode::SUCCESS
    }
}

#[cfg(not(feature = "qml"))]
fn main() -> std::process::ExitCode {
    eprintln!(
        "Qt/QML UI is disabled in this build. Rebuild with `--features qml` after installing Qt Quick development headers."
    );
    std::process::ExitCode::FAILURE
}

#[cfg(feature = "qml")]
fn main() -> std::process::ExitCode {
    qml_app::run()
}
