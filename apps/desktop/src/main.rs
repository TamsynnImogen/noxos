#[cfg(feature = "qml")]
mod qml_app {
    use qmetaobject::listmodel::QAbstractListModel;
    use qmetaobject::prelude::*;
    use std::collections::HashMap;
    use std::env;
    use std::fs;
    use std::io;
    use std::path::{Path, PathBuf};
    use std::process::{Command, ExitCode, Stdio};

    const ROLE_NAME: i32 = qmetaobject::USER_ROLE;
    const ROLE_PATH: i32 = qmetaobject::USER_ROLE + 1;
    const ROLE_URL: i32 = qmetaobject::USER_ROLE + 2;
    const ROLE_IS_DIR: i32 = qmetaobject::USER_ROLE + 3;

    #[derive(Clone, Default)]
    struct DesktopItem {
        name: QString,
        path: QString,
        url: QString,
        is_dir: bool,
    }

    #[derive(QObject, Default)]
    struct DesktopModel {
        #[qt_base_class = "QAbstractListModel"]
        base: qmetaobject::QObjectCppWrapper,
        entries: Vec<DesktopItem>,
        desktop_path: qt_property!(QString; NOTIFY desktop_path_changed),
        desktop_path_changed: qt_signal!(),
        error_message: qt_property!(QString; NOTIFY error_message_changed),
        error_message_changed: qt_signal!(),
        refresh: qt_method!(
            fn refresh(&mut self) {
                self.reload();
            }
        ),
        open_in_files: qt_method!(
            fn open_in_files(&mut self, path: QString) {
                let input_path = PathBuf::from(path.to_string());
                let target_path = files_target_directory(&input_path);

                match launch_files_app(&target_path) {
                    Ok(()) => {
                        self.error_message = QString::default();
                        self.error_message_changed();
                    }
                    Err(error) => {
                        self.error_message =
                            QString::from(format!("Failed to open Files: {error}"));
                        self.error_message_changed();
                    }
                }
            }
        ),
        launch_app: qt_method!(
            fn launch_app(&mut self, app_id: QString) {
                match launch_app_by_id(app_id.to_string().as_str()) {
                    Ok(()) => {
                        self.error_message = QString::default();
                        self.error_message_changed();
                    }
                    Err(error) => {
                        self.error_message =
                            QString::from(format!("Failed to launch app: {error}"));
                        self.error_message_changed();
                    }
                }
            }
        ),
    }

    impl QAbstractListModel for DesktopModel {
        fn row_count(&self) -> i32 {
            self.entries.len() as i32
        }

        fn data(&self, index: QModelIndex, role: i32) -> QVariant {
            let row = index.row();
            if row < 0 || row as usize >= self.entries.len() {
                return QVariant::default();
            }

            let item = &self.entries[row as usize];
            match role {
                ROLE_NAME => item.name.clone().into(),
                ROLE_PATH => item.path.clone().into(),
                ROLE_URL => item.url.clone().into(),
                ROLE_IS_DIR => item.is_dir.into(),
                _ => QVariant::default(),
            }
        }

        fn role_names(&self) -> HashMap<i32, QByteArray> {
            HashMap::from([
                (ROLE_NAME, QByteArray::from("name")),
                (ROLE_PATH, QByteArray::from("path")),
                (ROLE_URL, QByteArray::from("url")),
                (ROLE_IS_DIR, QByteArray::from("isDir")),
            ])
        }
    }

    impl DesktopModel {
        fn reload(&mut self) {
            let desktop_path = resolve_desktop_path();
            match read_desktop_entries(&desktop_path) {
                Ok(entries) => {
                    self.begin_reset_model();
                    self.entries = entries;
                    self.end_reset_model();
                    self.desktop_path = QString::from(desktop_path.display().to_string());
                    self.desktop_path_changed();
                    self.error_message = QString::default();
                    self.error_message_changed();
                }
                Err(error) => {
                    self.error_message =
                        QString::from(format!("Failed to load desktop items: {error}"));
                    self.error_message_changed();
                }
            }
        }
    }

    fn qml_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("qml/Main.qml")
    }

    fn workspace_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")))
    }

    fn resolve_desktop_path() -> PathBuf {
        let home = std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/"));
        home.join("Desktop")
    }

    fn path_to_file_url(path: &Path) -> String {
        let encoded = path
            .components()
            .enumerate()
            .map(|(index, component)| {
                let text = component.as_os_str().to_string_lossy();
                if index == 0 && text == "/" {
                    String::new()
                } else {
                    urlencoding::encode(&text).into_owned()
                }
            })
            .collect::<Vec<_>>()
            .join("/");
        format!("file://{encoded}")
    }

    fn read_desktop_entries(path: &Path) -> std::io::Result<Vec<DesktopItem>> {
        let mut entries = fs::read_dir(path)?
            .filter_map(|entry| entry.ok())
            .filter_map(|entry| {
                let name = entry.file_name().to_string_lossy().into_owned();
                if name.starts_with('.') {
                    return None;
                }
                let path = entry.path();
                let metadata = entry.metadata().ok()?;
                Some(DesktopItem {
                    name: QString::from(name),
                    path: QString::from(path.display().to_string()),
                    url: QString::from(path_to_file_url(&path)),
                    is_dir: metadata.is_dir(),
                })
            })
            .collect::<Vec<_>>();

        entries.sort_by(|left, right| {
            left.name
                .to_string()
                .to_lowercase()
                .cmp(&right.name.to_string().to_lowercase())
        });
        Ok(entries)
    }

    fn files_target_directory(path: &Path) -> PathBuf {
        if path.as_os_str().is_empty() {
            return resolve_desktop_path();
        }

        match fs::metadata(path) {
            Ok(metadata) if metadata.is_dir() => path.to_path_buf(),
            Ok(_) => path
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or_else(resolve_desktop_path),
            Err(_) => path.to_path_buf(),
        }
    }

    fn launch_files_app(path: &Path) -> io::Result<()> {
        if let Some(program) = env::var_os("SYSAPPS_FILES_APP") {
            spawn_detached(Command::new(program).arg(path))?;
            return Ok(());
        }

        let workspace = workspace_root();
        if workspace.join("Cargo.toml").exists() {
            spawn_detached(
                Command::new("cargo")
                    .current_dir(workspace)
                    .arg("run")
                    .arg("-p")
                    .arg("files-app")
                    .arg("--features")
                    .arg("qml")
                    .arg("--")
                    .arg(path),
            )?;
            return Ok(());
        }

        if let Ok(current_exe) = env::current_exe()
            && let Some(exe_dir) = current_exe.parent()
        {
            let sibling = exe_dir.join("files-app");
            if sibling.exists() {
                spawn_detached(Command::new(sibling).arg(path))?;
                return Ok(());
            }
        }

        spawn_detached(Command::new("files-app").arg(path))
    }

    fn launch_image_viewer(path: &Path) -> io::Result<()> {
        if let Some(program) = env::var_os("SYSAPPS_IMAGE_VIEWER_BIN") {
            spawn_detached(Command::new(program).arg(path))?;
            return Ok(());
        }

        let workspace = workspace_root();
        if workspace.join("Cargo.toml").exists() {
            spawn_detached(
                Command::new("cargo")
                    .current_dir(workspace)
                    .arg("run")
                    .arg("-p")
                    .arg("image-viewer-app")
                    .arg("--features")
                    .arg("qml")
                    .arg("--")
                    .arg(path),
            )?;
            return Ok(());
        }

        spawn_detached(Command::new("image-viewer-app").arg(path))
    }

    fn launch_app_by_id(app_id: &str) -> io::Result<()> {
        match app_id {
            "files" => launch_files_app(&resolve_desktop_path()),
            "terminal" => launch_terminal(),
            "browser" => spawn_first_available(&[
                CommandSpec::program("firefox"),
                CommandSpec::new("xdg-open", &["https://www.mozilla.org/firefox/"]),
            ]),
            "image-viewer" => launch_image_viewer(&resolve_desktop_path()),
            "settings" => spawn_first_available(&[
                CommandSpec::program("cinnamon-settings"),
                CommandSpec::program("gnome-control-center"),
                CommandSpec::program("systemsettings"),
            ]),
            "software" => spawn_first_available(&[
                CommandSpec::program("mintinstall"),
                CommandSpec::program("gnome-software"),
                CommandSpec::program("plasma-discover"),
            ]),
            "calculator" => spawn_first_available(&[
                CommandSpec::program("gnome-calculator"),
                CommandSpec::program("kcalc"),
                CommandSpec::program("qalculate-gtk"),
            ]),
            "lock" => spawn_first_available(&[
                CommandSpec::new("loginctl", &["lock-session"]),
                CommandSpec::new("xdg-screensaver", &["lock"]),
            ]),
            "sleep" => spawn_detached(Command::new("systemctl").arg("suspend")),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("unknown app id `{app_id}`"),
            )),
        }
    }

    fn launch_terminal() -> io::Result<()> {
        if let Some(program) = env::var_os("TERMINAL") {
            return spawn_detached(&mut Command::new(program));
        }

        spawn_first_available(&[
            CommandSpec::program("konsole"),
            CommandSpec::program("gnome-terminal"),
            CommandSpec::program("x-terminal-emulator"),
            CommandSpec::program("xterm"),
        ])
    }

    struct CommandSpec<'a> {
        program: &'a str,
        args: &'a [&'a str],
    }

    impl<'a> CommandSpec<'a> {
        fn program(program: &'a str) -> Self {
            Self { program, args: &[] }
        }

        fn new(program: &'a str, args: &'a [&'a str]) -> Self {
            Self { program, args }
        }
    }

    fn spawn_first_available(commands: &[CommandSpec<'_>]) -> io::Result<()> {
        let mut last_error = None;
        for spec in commands {
            let mut command = Command::new(spec.program);
            command.args(spec.args);
            match spawn_detached(&mut command) {
                Ok(()) => return Ok(()),
                Err(error) if error.kind() == io::ErrorKind::NotFound => {
                    last_error = Some(error);
                }
                Err(error) => return Err(error),
            }
        }

        Err(last_error.unwrap_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "no launcher command was configured",
            )
        }))
    }

    fn spawn_detached(command: &mut Command) -> io::Result<()> {
        command
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map(|_| ())
    }

    pub fn run() -> ExitCode {
        let mut engine = QmlEngine::new();
        let model = QObjectBox::new(DesktopModel::default());

        {
            let pinned_model = model.pinned();
            let mut pinned = pinned_model.borrow_mut();
            pinned.reload();
        }

        engine.set_object_property(QString::from("desktopModel"), model.pinned());
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
