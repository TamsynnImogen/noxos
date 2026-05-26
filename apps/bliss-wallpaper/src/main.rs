#[cfg(feature = "qml")]
mod qml_app {
    use qmetaobject::prelude::*;
    use std::path::PathBuf;
    use std::process::ExitCode;

    fn qml_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("qml/Main.qml")
    }

    pub fn run() -> ExitCode {
        let mut engine = QmlEngine::new();
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
