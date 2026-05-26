#[cfg(feature = "qml")]
mod qml_app {
    use file_ops::{
        EntryKind, FileEntry, GroupMode, ListOptions, SortField, group_label, list_directory,
    };
    use qmetaobject::listmodel::QAbstractListModel;
    use qmetaobject::prelude::*;
    use serde::{Deserialize, Serialize};
    use std::borrow::Cow;
    use std::collections::hash_map::DefaultHasher;
    use std::collections::{HashMap, HashSet};
    use std::env;
    use std::ffi::OsStr;
    use std::fmt::Write as _;
    use std::fs;
    use std::hash::{Hash, Hasher};
    use std::io::{self, Read, Write};
    use std::path::{Path, PathBuf};
    use std::process::{Child, Command, Stdio};
    use std::sync::{
        Arc, Mutex, OnceLock,
        atomic::{AtomicUsize, Ordering},
    };
    use std::thread;
    use std::time::{Instant, SystemTime, UNIX_EPOCH};

    const ROLE_NAME: i32 = qmetaobject::USER_ROLE;
    const ROLE_PATH: i32 = qmetaobject::USER_ROLE + 1;
    const ROLE_KIND: i32 = qmetaobject::USER_ROLE + 2;
    const ROLE_SIZE_TEXT: i32 = qmetaobject::USER_ROLE + 3;
    const ROLE_MODIFIED_TEXT: i32 = qmetaobject::USER_ROLE + 4;
    const ROLE_HIDDEN: i32 = qmetaobject::USER_ROLE + 5;
    const ROLE_IS_DIRECTORY: i32 = qmetaobject::USER_ROLE + 6;
    const ROLE_MODIFIED_MS: i32 = qmetaobject::USER_ROLE + 7;
    const ROLE_GROUP_LABEL: i32 = qmetaobject::USER_ROLE + 8;
    const ROLE_GROUP_START: i32 = qmetaobject::USER_ROLE + 9;
    const ROLE_DEVICE_USAGE: i32 = qmetaobject::USER_ROLE + 10;
    const ROLE_THUMBNAIL_URL: i32 = qmetaobject::USER_ROLE + 11;
    const FAVORITES_URI: &str = "virtual://favorites";
    const DEVICES_URI: &str = "virtual://devices";
    const ARCHIVE_URI_PREFIX: &str = "archive://";
    const GNOME_COPIED_FILES_MIME: &str = "x-special/gnome-copied-files";
    const URI_LIST_MIME: &str = "text/uri-list";
    const DEFAULT_VIEW_MODE: &str = "Details";
    const DEFAULT_SORT_FIELD: &str = "Name";
    const DEFAULT_GROUPING: &str = "None";

    #[derive(Clone, Default)]
    struct FileItem {
        name: QString,
        path: QString,
        kind: QString,
        size_text: QString,
        modified_text: QString,
        modified_ms: u64,
        thumbnail_url: QString,
        group_label: QString,
        group_start: bool,
        hidden: bool,
        is_directory: bool,
    }

    #[derive(Clone, Default)]
    struct DeviceItem {
        label: QString,
        mount_path: QString,
        details: QString,
        usage_percent: f64,
        mounted: bool,
    }

    #[derive(Clone, Debug, Deserialize, Serialize)]
    struct FolderSettings {
        view_mode: String,
        sort_field: String,
        sort_descending: bool,
        grouping: String,
        show_hidden: bool,
    }

    impl Default for FolderSettings {
        fn default() -> Self {
            Self {
                view_mode: DEFAULT_VIEW_MODE.to_string(),
                sort_field: DEFAULT_SORT_FIELD.to_string(),
                sort_descending: false,
                grouping: DEFAULT_GROUPING.to_string(),
                show_hidden: false,
            }
        }
    }

    #[derive(Clone, Debug, Default, Deserialize, Serialize)]
    struct FolderSettingsStore {
        folders: HashMap<String, FolderSettings>,
    }

    #[derive(Clone, Debug, Eq, Hash, PartialEq)]
    struct ArchiveCacheKey {
        path: PathBuf,
        size_bytes: u64,
        modified_secs: u64,
        modified_nanos: u32,
    }

    #[derive(Clone, Debug)]
    struct ArchiveRawEntry {
        path: String,
        folder: bool,
        size_bytes: u64,
        modified: String,
    }

    #[derive(Clone, Debug)]
    struct LoadedEntry {
        name: String,
        path: String,
        kind: EntryKind,
        size_text: String,
        modified_text: String,
        modified_ms: u64,
        thumbnail_url: String,
        group_label: String,
        hidden: bool,
        is_directory: bool,
    }

    struct LoadResult {
        generation: usize,
        path: String,
        history_path: PathBuf,
        record_history: bool,
        group_mode: GroupMode,
        result: Result<Vec<LoadedEntry>, String>,
    }

    struct OperationProgress {
        title: String,
        detail: String,
        bytes_done: u64,
        bytes_total: u64,
        bytes_per_second: f64,
    }

    struct OperationResult {
        title: String,
        cut: bool,
        clear_desktop_clipboard: bool,
        result: Result<(), String>,
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum PasteConflictMode {
        Rename,
        Overwrite,
    }

    impl PasteConflictMode {
        fn from_qml(value: &str) -> Self {
            match value {
                "overwrite" => Self::Overwrite,
                _ => Self::Rename,
            }
        }
    }

    #[derive(QObject, Default)]
    struct FilesModel {
        #[qt_base_class = "QAbstractListModel"]
        base: qmetaobject::QObjectCppWrapper,
        entries: Vec<FileItem>,
        current_path: qt_property!(QString; NOTIFY current_path_changed),
        current_path_changed: qt_signal!(),
        error_message: qt_property!(QString; NOTIFY error_message_changed),
        error_message_changed: qt_signal!(),
        show_hidden: qt_property!(bool; NOTIFY show_hidden_changed),
        show_hidden_changed: qt_signal!(),
        sort_field_name: qt_property!(QString; NOTIFY sort_field_name_changed),
        sort_field_name_changed: qt_signal!(),
        sort_descending: qt_property!(bool; NOTIFY sort_descending_changed),
        sort_descending_changed: qt_signal!(),
        grouping_name: qt_property!(QString; NOTIFY grouping_name_changed),
        grouping_name_changed: qt_signal!(),
        folder_view_mode: qt_property!(QString; NOTIFY folder_view_mode_changed),
        folder_view_mode_changed: qt_signal!(),
        grouped_entries_json: qt_property!(QString; NOTIFY grouped_entries_json_changed),
        grouped_entries_json_changed: qt_signal!(),
        can_go_back: qt_property!(bool; NOTIFY can_go_back_changed),
        can_go_back_changed: qt_signal!(),
        can_go_forward: qt_property!(bool; NOTIFY can_go_forward_changed),
        can_go_forward_changed: qt_signal!(),
        selected_info_name: qt_property!(QString; NOTIFY selected_info_changed),
        selected_info_kind: qt_property!(QString; NOTIFY selected_info_changed),
        selected_info_size: qt_property!(QString; NOTIFY selected_info_changed),
        selected_info_modified: qt_property!(QString; NOTIFY selected_info_changed),
        selected_info_modified_ms: qt_property!(f64; NOTIFY selected_info_changed),
        selected_info_path: qt_property!(QString; NOTIFY selected_info_changed),
        selected_info_thumbnail_url: qt_property!(QString; NOTIFY selected_info_changed),
        selected_info_is_directory: qt_property!(bool; NOTIFY selected_info_changed),
        selected_info_changed: qt_signal!(),
        terminal_output: qt_property!(QString; NOTIFY terminal_output_changed),
        terminal_output_changed: qt_signal!(),
        can_paste: qt_property!(bool; NOTIFY can_paste_changed),
        can_paste_changed: qt_signal!(),
        operation_active: qt_property!(bool; NOTIFY operation_changed),
        operation_title: qt_property!(QString; NOTIFY operation_changed),
        operation_detail: qt_property!(QString; NOTIFY operation_changed),
        operation_completed: qt_property!(f64; NOTIFY operation_changed),
        operation_total: qt_property!(f64; NOTIFY operation_changed),
        operation_speed: qt_property!(f64; NOTIFY operation_changed),
        operation_history_json: qt_property!(QString; NOTIFY operation_changed),
        operation_changed: qt_signal!(),
        thumbnail_jobs: Arc<AtomicUsize>,
        queued_thumbnails: Arc<Mutex<HashSet<PathBuf>>>,
        load_generation: Arc<AtomicUsize>,
        clipboard_paths: Vec<PathBuf>,
        clipboard_cut: bool,
        desktop_clipboard_owner: Option<Child>,
        history: Vec<PathBuf>,
        history_index: usize,
        load_path: qt_method!(
            fn load_path(&mut self, path: QString) {
                self.load_path_impl(normalize_path_input(&path.to_string()));
            }
        ),
        refresh: qt_method!(
            fn refresh(&mut self) {
                self.reload_current_directory();
            }
        ),
        go_up: qt_method!(
            fn go_up(&mut self) {
                let current_text = self.current_path.to_string();
                if let Some((archive_path, inner_path)) = parse_archive_uri(&current_text) {
                    if inner_path.is_empty() {
                        if let Some(parent) = archive_path.parent() {
                            self.load_path_impl(parent.to_path_buf());
                        }
                    } else {
                        let parent_inner = archive_parent_path(&inner_path);
                        self.load_path_impl(PathBuf::from(archive_uri(
                            &archive_path,
                            &parent_inner,
                        )));
                    }
                    return;
                }

                if is_virtual_location(current_text.as_str()) {
                    return;
                }
                let current = PathBuf::from(self.current_path.to_string());
                if let Some(parent) = current.parent() {
                    self.load_path_impl(parent.to_path_buf());
                }
            }
        ),
        go_back: qt_method!(
            fn go_back(&mut self) {
                if self.history_index > 0 {
                    self.history_index -= 1;
                    let path = self.history[self.history_index].clone();
                    self.load_path_without_history(path);
                    self.update_history_flags();
                }
            }
        ),
        go_forward: qt_method!(
            fn go_forward(&mut self) {
                if self.history_index + 1 < self.history.len() {
                    self.history_index += 1;
                    let path = self.history[self.history_index].clone();
                    self.load_path_without_history(path);
                    self.update_history_flags();
                }
            }
        ),
        set_show_hidden: qt_method!(
            fn set_show_hidden(&mut self, show_hidden: bool) {
                self.show_hidden = show_hidden;
                self.show_hidden_changed();
                self.save_current_folder_settings();
                self.reload_current_directory();
            }
        ),
        set_sort_field: qt_method!(
            fn set_sort_field(&mut self, field_name: QString) {
                self.sort_field_name = field_name.clone();
                self.sort_field_name_changed();
                self.save_current_folder_settings();
                self.reload_current_directory();
            }
        ),
        set_sort_descending: qt_method!(
            fn set_sort_descending(&mut self, descending: bool) {
                self.sort_descending = descending;
                self.sort_descending_changed();
                self.save_current_folder_settings();
                self.reload_current_directory();
            }
        ),
        set_grouping: qt_method!(
            fn set_grouping(&mut self, grouping_name: QString) {
                self.grouping_name = grouping_name;
                self.grouping_name_changed();
                self.save_current_folder_settings();
                self.reload_current_directory();
            }
        ),
        activate_entry: qt_method!(
            fn activate_entry(&mut self, path: QString, is_directory: bool) {
                let normalized_path = normalize_path_input(&path.to_string());
                if is_directory {
                    self.load_path_impl(normalized_path);
                    return;
                }

                if let Some((archive_path, inner_path)) =
                    parse_archive_uri(&normalized_path.to_string_lossy())
                {
                    let result = if is_supported_media(Path::new(&inner_path)) {
                        open_in_image_viewer(&archive_uri(&archive_path, &inner_path))
                            .or_else(|_| open_archive_member(&archive_path, &inner_path))
                    } else {
                        open_archive_member(&archive_path, &inner_path)
                    };

                    if let Err(error) = result {
                        self.error_message =
                            QString::from(format!("Failed to open archive item: {error}"));
                        self.error_message_changed();
                    }
                    return;
                }

                if is_archive_extension(&normalized_path) {
                    self.load_path_impl(PathBuf::from(archive_uri(&normalized_path, "")));
                    return;
                }

                if normalized_path.as_os_str().is_empty()
                    || is_virtual_location(&normalized_path.to_string_lossy())
                {
                    return;
                }

                if is_supported_media(&normalized_path) {
                    let result = open_in_image_viewer(&normalized_path.display().to_string())
                        .or_else(|_| open_with_default(&normalized_path));

                    if let Err(error) = result {
                        self.error_message =
                            QString::from(format!("Failed to open image: {error}"));
                        self.error_message_changed();
                    }
                    return;
                }

                if let Err(error) = open_with_default(&normalized_path) {
                    self.error_message = QString::from(format!("Failed to open item: {error}"));
                    self.error_message_changed();
                }
            }
        ),
        move_to_trash: qt_method!(
            fn move_to_trash(&mut self, path: QString) {
                let normalized_path = normalize_path_input(&path.to_string());
                if is_read_only_location(&self.current_path.to_string())
                    || is_read_only_location(&normalized_path.to_string_lossy())
                {
                    self.error_message = QString::from("Virtual folders cannot be modified");
                    self.error_message_changed();
                    return;
                }
                match trash_path(&normalized_path) {
                    Ok(()) => {
                        self.reload_current_directory();
                    }
                    Err(error) => {
                        self.error_message =
                            QString::from(format!("Failed to move item to trash: {error}"));
                        self.error_message_changed();
                    }
                }
            }
        ),
        move_paths_to_trash: qt_method!(
            fn move_paths_to_trash(&mut self, paths_json: QString) {
                if is_read_only_location(&self.current_path.to_string()) {
                    self.error_message = QString::from("Virtual folders cannot be modified");
                    self.error_message_changed();
                    return;
                }

                let paths = match decode_paths_json(&paths_json.to_string(), "selection did not contain any local file paths") {
                    Ok(paths) => paths,
                    Err(error) => {
                        self.error_message =
                            QString::from(format!("Failed to read selected items: {error}"));
                        self.error_message_changed();
                        return;
                    }
                };

                for path in paths {
                    if is_read_only_location(&path.to_string_lossy()) {
                        self.error_message = QString::from("Read-only items cannot be trashed");
                        self.error_message_changed();
                        return;
                    }
                    if let Err(error) = trash_path(&path) {
                        self.error_message =
                            QString::from(format!("Failed to move item to trash: {error}"));
                        self.error_message_changed();
                        return;
                    }
                }

                self.reload_current_directory();
            }
        ),
        create_folder: qt_method!(
            fn create_folder(&mut self, name: QString) {
                let name = name.to_string();
                if is_read_only_location(&self.current_path.to_string()) {
                    self.error_message = QString::from("Virtual folders cannot contain new items");
                    self.error_message_changed();
                    return;
                }
                match create_directory_in_current(Path::new(&self.current_path.to_string()), &name)
                {
                    Ok(()) => self.reload_current_directory(),
                    Err(error) => {
                        self.error_message =
                            QString::from(format!("Failed to create folder: {error}"));
                        self.error_message_changed();
                    }
                }
            }
        ),
        create_file: qt_method!(
            fn create_file(&mut self, name: QString) {
                let name = name.to_string();
                if is_read_only_location(&self.current_path.to_string()) {
                    self.error_message = QString::from("Virtual folders cannot contain new items");
                    self.error_message_changed();
                    return;
                }
                match create_file_in_current(Path::new(&self.current_path.to_string()), &name) {
                    Ok(()) => self.reload_current_directory(),
                    Err(error) => {
                        self.error_message =
                            QString::from(format!("Failed to create file: {error}"));
                        self.error_message_changed();
                    }
                }
            }
        ),
        rename_path: qt_method!(
            fn rename_path(&mut self, path: QString, new_name: QString) {
                let path = normalize_path_input(&path.to_string());
                let new_name = new_name.to_string();
                if is_read_only_location(&self.current_path.to_string())
                    || is_read_only_location(&path.to_string_lossy())
                {
                    self.error_message =
                        QString::from("Virtual folder shortcuts cannot be renamed");
                    self.error_message_changed();
                    return;
                }
                match rename_item(&path, &new_name) {
                    Ok(()) => self.reload_current_directory(),
                    Err(error) => {
                        self.error_message =
                            QString::from(format!("Failed to rename item: {error}"));
                        self.error_message_changed();
                    }
                }
            }
        ),
        copy_path: qt_method!(
            fn copy_path(&mut self, path: QString) {
                self.set_file_clipboard(normalize_path_input(&path.to_string()), false);
            }
        ),
        cut_path: qt_method!(
            fn cut_path(&mut self, path: QString) {
                self.set_file_clipboard(normalize_path_input(&path.to_string()), true);
            }
        ),
        copy_paths: qt_method!(
            fn copy_paths(&mut self, paths_json: QString) {
                match decode_paths_json(&paths_json.to_string(), "selection did not contain any local file paths") {
                    Ok(paths) => self.set_file_clipboard_paths(paths, false),
                    Err(error) => {
                        self.error_message =
                            QString::from(format!("Failed to read selected items: {error}"));
                        self.error_message_changed();
                    }
                }
            }
        ),
        cut_paths: qt_method!(
            fn cut_paths(&mut self, paths_json: QString) {
                match decode_paths_json(&paths_json.to_string(), "selection did not contain any local file paths") {
                    Ok(paths) => self.set_file_clipboard_paths(paths, true),
                    Err(error) => {
                        self.error_message =
                            QString::from(format!("Failed to read selected items: {error}"));
                        self.error_message_changed();
                    }
                }
            }
        ),
        paste_into_current: qt_method!(
            fn paste_into_current(&mut self) {
                self.paste_into_current_with_conflict_mode(QString::from("rename"));
            }
        ),
        paste_into_current_with_conflict_mode: qt_method!(
            fn paste_into_current_with_conflict_mode(&mut self, conflict_mode: QString) {
                let current_path = self.current_path.to_string();
                if is_read_only_location(&current_path) {
                    self.error_message = QString::from("This folder cannot be modified");
                    self.error_message_changed();
                    return;
                }

                let destination = PathBuf::from(&current_path);
                let (sources, cut, desktop_clipboard_was_used) = match self.clipboard_sources() {
                    Ok(Some(clipboard)) => clipboard,
                    Ok(None) => return,
                    Err(error) => {
                        self.error_message =
                            QString::from(format!("Failed to read desktop clipboard: {error}"));
                        self.error_message_changed();
                        return;
                    }
                };

                let mode = PasteConflictMode::from_qml(conflict_mode.to_string().as_str());
                self.start_paste_operation(sources, destination, cut, desktop_clipboard_was_used, mode);
            }
        ),
        paste_conflicts_json: qt_method!(
            fn paste_conflicts_json(&self) -> QString {
                let current_path = self.current_path.to_string();
                if is_read_only_location(&current_path) {
                    return QString::from("{\"count\":0,\"names\":[]}");
                }

                let destination = PathBuf::from(&current_path);
                let (sources, _, _) = match self.clipboard_sources() {
                    Ok(Some(clipboard)) => clipboard,
                    Ok(None) | Err(_) => return QString::from("{\"count\":0,\"names\":[]}"),
                };

                let names = paste_conflict_names(&sources, &destination);
                let payload = serde_json::json!({
                    "count": names.len(),
                    "names": names,
                });
                QString::from(payload.to_string())
            }
        ),
        drop_paths_into_directory: qt_method!(
            fn drop_paths_into_directory(
                &mut self,
                paths_json: QString,
                destination_dir: QString,
                copy: bool,
            ) {
                self.drop_paths_into_directory_with_conflict_mode(
                    paths_json,
                    destination_dir,
                    copy,
                    QString::from("rename"),
                );
            }
        ),
        drop_paths_into_directory_with_conflict_mode: qt_method!(
            fn drop_paths_into_directory_with_conflict_mode(
                &mut self,
                paths_json: QString,
                destination_dir: QString,
                copy: bool,
                conflict_mode: QString,
            ) {
                let destination = normalize_path_input(&destination_dir.to_string());
                if is_read_only_location(&destination.to_string_lossy()) {
                    self.error_message = QString::from("This folder cannot be modified");
                    self.error_message_changed();
                    return;
                }

                let sources = match decode_paths_json(&paths_json.to_string(), "drop did not contain any local file paths") {
                    Ok(sources) => sources,
                    Err(error) => {
                        self.error_message =
                            QString::from(format!("Failed to read dropped item: {error}"));
                        self.error_message_changed();
                        return;
                    }
                };

                if let Err(error) = validate_transfer_items_to_directory(&sources, &destination) {
                    self.error_message = QString::from(format!("Failed to drop item: {error}"));
                    self.error_message_changed();
                    return;
                }

                let mode = PasteConflictMode::from_qml(conflict_mode.to_string().as_str());
                self.start_paste_operation(sources, destination, !copy, false, mode);
            }
        ),
        drop_conflicts_json: qt_method!(
            fn drop_conflicts_json(&self, paths_json: QString, destination_dir: QString) -> QString {
                let destination = normalize_path_input(&destination_dir.to_string());
                if is_read_only_location(&destination.to_string_lossy()) {
                    return QString::from("{\"count\":0,\"names\":[]}");
                }

                let sources = match decode_paths_json(&paths_json.to_string(), "drop did not contain any local file paths") {
                    Ok(sources) => sources,
                    Err(_) => return QString::from("{\"count\":0,\"names\":[]}"),
                };

                let names = paste_conflict_names(&sources, &destination);
                let payload = serde_json::json!({
                    "count": names.len(),
                    "names": names,
                });
                QString::from(payload.to_string())
            }
        ),
        set_selected_path: qt_method!(
            fn set_selected_path(&mut self, path: QString) {
                self.update_selected_info(path.to_string().as_str());
            }
        ),
        desktop_clipboard_has_files: qt_method!(
            fn desktop_clipboard_has_files(&self) -> bool {
                desktop_file_clipboard_has_files()
            }
        ),
        current_paths_json: qt_method!(
            fn current_paths_json(&self) -> QString {
                let paths: Vec<String> = self
                    .entries
                    .iter()
                    .map(|entry| entry.path.to_string())
                    .filter(|path| !path.is_empty() && !is_read_only_location(path))
                    .collect();
                QString::from(serde_json::to_string(&paths).unwrap_or_else(|_| "[]".to_string()))
            }
        ),
        execute_terminal_command: qt_method!(
            fn execute_terminal_command(&mut self, command: QString) {
                let command_text = command.to_string();
                let trimmed = command_text.trim();
                if trimmed.is_empty() {
                    return;
                }

                let cwd = self.terminal_working_directory();
                let mut transcript = self.terminal_output.to_string();
                let prompt_path = cwd.display().to_string();
                let _ = writeln!(&mut transcript, "{} $ {}", prompt_path, trimmed);

                match run_shell_command(trimmed, &cwd) {
                    Ok(output) => {
                        if !output.is_empty() {
                            let _ = writeln!(&mut transcript, "{output}");
                        }
                    }
                    Err(error) => {
                        let _ = writeln!(&mut transcript, "Error: {error}");
                    }
                }

                if !transcript.ends_with('\n') {
                    transcript.push('\n');
                }

                self.terminal_output = QString::from(transcript);
                self.terminal_output_changed();
            }
        ),
        clear_terminal_output: qt_method!(
            fn clear_terminal_output(&mut self) {
                self.terminal_output = QString::default();
                self.terminal_output_changed();
            }
        ),
        open_terminal: qt_method!(
            fn open_terminal(&mut self) {
                let cwd = self.terminal_working_directory();
                match open_konsole(&cwd) {
                    Ok(()) => {
                        self.error_message = QString::default();
                        self.error_message_changed();
                    }
                    Err(error) => {
                        self.error_message =
                            QString::from(format!("Failed to open Konsole: {error}"));
                        self.error_message_changed();
                    }
                }
            }
        ),
        pop_out_terminal: qt_method!(
            fn pop_out_terminal(&mut self) {
                let cwd = self.terminal_working_directory();
                match open_noxcmd(&cwd) {
                    Ok(()) => {
                        self.error_message = QString::default();
                        self.error_message_changed();
                    }
                    Err(error) => {
                        self.error_message =
                            QString::from(format!("Failed to open NOXcmd: {error}"));
                        self.error_message_changed();
                    }
                }
            }
        ),
        set_folder_view_mode: qt_method!(
            fn set_folder_view_mode(&mut self, view_mode: QString) {
                if self.folder_view_mode == view_mode {
                    return;
                }

                self.folder_view_mode = view_mode;
                self.folder_view_mode_changed();
                self.save_current_folder_settings();
            }
        ),
        thumbnail_jobs_pending: qt_method!(
            fn thumbnail_jobs_pending(&self) -> i32 {
                self.thumbnail_jobs.load(Ordering::SeqCst) as i32
            }
        ),
    }

    impl FilesModel {
        fn sort_field(&self) -> SortField {
            match self.sort_field_name.to_string().as_str() {
                "Size" => SortField::Size,
                "Type" => SortField::Kind,
                "Modified" => SortField::Modified,
                _ => SortField::Name,
            }
        }

        fn group_mode(&self) -> GroupMode {
            match self.grouping_name.to_string().as_str() {
                "Type" => GroupMode::Type,
                _ => GroupMode::None,
            }
        }

        fn apply_folder_settings(&mut self, folder_key: &str) {
            let settings = read_folder_settings(folder_key).unwrap_or_default();
            self.set_folder_settings_properties(settings);
        }

        fn set_folder_settings_properties(&mut self, settings: FolderSettings) {
            self.folder_view_mode = QString::from(settings.view_mode);
            self.folder_view_mode_changed();
            self.sort_field_name = QString::from(settings.sort_field);
            self.sort_field_name_changed();
            self.sort_descending = settings.sort_descending;
            self.sort_descending_changed();
            self.grouping_name = QString::from(settings.grouping);
            self.grouping_name_changed();
            self.show_hidden = settings.show_hidden;
            self.show_hidden_changed();
        }

        fn current_folder_settings(&self) -> FolderSettings {
            FolderSettings {
                view_mode: self.folder_view_mode.to_string(),
                sort_field: self.sort_field_name.to_string(),
                sort_descending: self.sort_descending,
                grouping: self.grouping_name.to_string(),
                show_hidden: self.show_hidden,
            }
        }

        fn save_current_folder_settings(&self) {
            let current_path = self.current_path.to_string();
            if current_path.is_empty() {
                return;
            }

            let _ = write_folder_settings(&current_path, self.current_folder_settings());
        }

        fn set_file_clipboard(&mut self, path: PathBuf, cut: bool) {
            self.set_file_clipboard_paths(vec![path], cut);
        }

        fn set_file_clipboard_paths(&mut self, paths: Vec<PathBuf>, cut: bool) {
            if paths.is_empty()
                || paths.iter().any(|path| {
                    path.as_os_str().is_empty() || is_read_only_location(&path.to_string_lossy())
                })
            {
                self.error_message = QString::from("This item cannot be copied");
                self.error_message_changed();
                return;
            }

            self.clipboard_paths.clear();
            self.clipboard_paths.extend(paths);
            self.clipboard_cut = cut;
            match write_desktop_file_clipboard(&self.clipboard_paths, cut) {
                Ok(owner) => self.replace_desktop_clipboard_owner(owner),
                Err(error) => {
                    self.error_message =
                        QString::from(format!("Using app clipboard only: {error}"));
                    self.error_message_changed();
                }
            }
            if !self.can_paste {
                self.can_paste = true;
                self.can_paste_changed();
            }
        }

        fn clear_file_clipboard(&mut self) {
            self.clipboard_paths.clear();
            self.clipboard_cut = false;
            self.stop_desktop_clipboard_owner();
            if self.can_paste {
                self.can_paste = false;
                self.can_paste_changed();
            }
        }

        fn replace_desktop_clipboard_owner(&mut self, owner: Child) {
            self.stop_desktop_clipboard_owner();
            self.desktop_clipboard_owner = Some(owner);
        }

        fn stop_desktop_clipboard_owner(&mut self) {
            if let Some(mut owner) = self.desktop_clipboard_owner.take() {
                let _ = owner.kill();
                let _ = owner.wait();
            }
        }

        fn clipboard_sources(&self) -> io::Result<Option<(Vec<PathBuf>, bool, bool)>> {
            match read_desktop_file_clipboard()? {
                Some((paths, cut)) => Ok(Some((paths, cut, true))),
                None => {
                    if self.clipboard_paths.is_empty() {
                        Ok(None)
                    } else {
                        Ok(Some((self.clipboard_paths.clone(), self.clipboard_cut, false)))
                    }
                }
            }
        }

        fn start_paste_operation(
            &mut self,
            sources: Vec<PathBuf>,
            destination: PathBuf,
            cut: bool,
            desktop_clipboard_was_used: bool,
            conflict_mode: PasteConflictMode,
        ) {
            if self.operation_active {
                self.error_message = QString::from("Another file operation is already running");
                self.error_message_changed();
                return;
            }

            let title = if cut { "Moving items" } else { "Copying items" }.to_string();
            let total_bytes = total_size_bytes(&sources);
            self.operation_active = true;
            self.operation_title = QString::from(title.clone());
            self.operation_detail = QString::from("Preparing...");
            self.operation_completed = 0.0;
            self.operation_total = total_bytes as f64;
            self.operation_speed = 0.0;
            self.operation_history_json = QString::from("[]");
            self.operation_changed();

            let qptr = QPointer::from(&*self);
            let apply_progress = qmetaobject::queued_callback(move |progress: OperationProgress| {
                if let Some(model) = qptr.as_pinned() {
                    model.borrow_mut().apply_operation_progress(progress);
                }
            });

            let qptr = QPointer::from(&*self);
            let apply_result = qmetaobject::queued_callback(move |result: OperationResult| {
                if let Some(model) = qptr.as_pinned() {
                    model.borrow_mut().apply_operation_result(result);
                }
            });

            thread::spawn(move || {
                let operation_title = title.clone();
                let result = paste_clipboard_items(
                    &sources,
                    &destination,
                    cut,
                    conflict_mode,
                    total_bytes,
                    |bytes_done, bytes_total, bytes_per_second, path| {
                        let detail = path
                            .file_name()
                            .map(|name| name.to_string_lossy().to_string())
                            .unwrap_or_else(|| path.display().to_string());
                        apply_progress(OperationProgress {
                            title: operation_title.clone(),
                            detail,
                            bytes_done,
                            bytes_total,
                            bytes_per_second,
                        });
                    },
                )
                .map_err(|error| format!("Failed to paste item: {error}"));

                apply_result(OperationResult {
                    title,
                    cut,
                    clear_desktop_clipboard: desktop_clipboard_was_used,
                    result,
                });
            });
        }

        fn apply_operation_progress(&mut self, progress: OperationProgress) {
            self.operation_active = true;
            self.operation_title = QString::from(progress.title);
            self.operation_detail = QString::from(progress.detail);
            self.operation_completed = progress.bytes_done as f64;
            self.operation_total = progress.bytes_total as f64;
            self.operation_speed = progress.bytes_per_second;
            self.push_operation_history(progress.bytes_per_second);
            self.operation_changed();
        }

        fn apply_operation_result(&mut self, result: OperationResult) {
            self.operation_active = false;
            self.operation_title = QString::from(result.title);
            self.operation_completed = self.operation_total;
            self.operation_speed = 0.0;
            match result.result {
                Ok(()) => {
                    self.operation_detail = QString::from("Complete");
                    if result.cut {
                        self.clear_file_clipboard();
                        if result.clear_desktop_clipboard {
                            let _ = clear_desktop_file_clipboard();
                        }
                    }
                    self.reload_current_directory();
                }
                Err(error) => {
                    self.operation_detail = QString::from("Failed");
                    self.error_message = QString::from(error);
                    self.error_message_changed();
                }
            }
            self.operation_changed();
        }

        fn push_operation_history(&mut self, bytes_per_second: f64) {
            let mut values: Vec<f64> =
                serde_json::from_str(&self.operation_history_json.to_string())
                    .unwrap_or_default();
            values.push(bytes_per_second.max(0.0));
            if values.len() > 48 {
                values.drain(0..values.len() - 48);
            }
            self.operation_history_json =
                QString::from(serde_json::to_string(&values).unwrap_or_else(|_| "[]".to_string()));
        }

        fn load_path_impl(&mut self, path: PathBuf) {
            self.load_path_inner(path, true);
        }

        fn load_path_without_history(&mut self, path: PathBuf) {
            self.load_path_inner(path, false);
        }

        fn load_path_inner(&mut self, path: PathBuf, record_history: bool) {
            let path_text = path.to_string_lossy();
            if let Some((archive_path, inner_path)) = parse_archive_uri(&path_text) {
                self.queue_archive_load(archive_path, inner_path, record_history);
                return;
            }

            if is_virtual_location(&path_text) {
                self.load_virtual_path(path_text.as_ref(), record_history);
                return;
            }

            let resolved_path = resolve_display_path(&path);
            let settings_key = resolved_path.display().to_string();
            self.apply_folder_settings(&settings_key);
            self.current_path = QString::from(settings_key.clone());
            self.current_path_changed();
            self.clear_entries_for_load();

            let generation = self.next_load_generation();
            let options = ListOptions {
                include_hidden: self.show_hidden,
                sort_field: self.sort_field(),
                directories_first: true,
                descending: self.sort_descending,
                group_mode: self.group_mode(),
            };
            let group_mode = self.group_mode();
            let thumbnail_jobs = Arc::clone(&self.thumbnail_jobs);
            let queued_thumbnails = Arc::clone(&self.queued_thumbnails);
            let qptr = QPointer::from(&*self);
            let apply_result = qmetaobject::queued_callback(move |result: LoadResult| {
                if let Some(model) = qptr.as_pinned() {
                    model.borrow_mut().apply_load_result(result);
                }
            });
            thread::spawn(move || {
                let result = list_directory(&resolved_path, options)
                    .map(|entries| {
                        entries
                            .into_iter()
                            .map(|entry| {
                                LoadedEntry::from_entry(
                                    entry,
                                    group_mode,
                                    &thumbnail_jobs,
                                    &queued_thumbnails,
                                )
                            })
                            .collect()
                    })
                    .map_err(|error| {
                        format!("Failed to list {}: {error}", resolved_path.display())
                    });

                apply_result(LoadResult {
                    generation,
                    path: settings_key,
                    history_path: resolved_path,
                    record_history,
                    group_mode,
                    result,
                });
            });
        }

        fn load_virtual_path(&mut self, location: &str, record_history: bool) {
            self.apply_folder_settings(location);

            let mut items =
                match list_virtual_entries(location, self.group_mode() != GroupMode::None) {
                    Ok(items) => items,
                    Err(error) => {
                        self.error_message =
                            QString::from(format!("Failed to list {location}: {error}"));
                        self.error_message_changed();
                        return;
                    }
                };

            sort_virtual_items(
                &mut items,
                self.sort_field_name.to_string().as_str(),
                self.sort_descending,
                self.group_mode() != GroupMode::None,
            );

            self.begin_reset_model();
            let mut previous_group = String::new();
            for item in &mut items {
                let current_group = item.group_label.to_string();
                item.group_start = !current_group.is_empty() && current_group != previous_group;
                previous_group = current_group;
            }
            self.grouped_entries_json = if self.group_mode() == GroupMode::None {
                QString::default()
            } else {
                QString::from(build_grouped_entries_json(&items))
            };
            self.grouped_entries_json_changed();
            self.entries = items;
            self.end_reset_model();
            self.current_path = QString::from(location);
            self.current_path_changed();
            self.set_folder_info();
            self.error_message = QString::default();
            self.error_message_changed();
            if record_history {
                self.push_history(PathBuf::from(location));
            } else {
                self.update_history_flags();
            }
        }

        fn queue_archive_load(
            &mut self,
            archive_path: PathBuf,
            inner_path: String,
            record_history: bool,
        ) {
            let settings_key = archive_uri(&archive_path, &inner_path);
            self.apply_folder_settings(&settings_key);
            self.current_path = QString::from(settings_key.clone());
            self.current_path_changed();
            self.clear_entries_for_load();

            let generation = self.next_load_generation();
            let group_mode = self.group_mode();
            let sort_field_name = self.sort_field_name.to_string();
            let sort_descending = self.sort_descending;
            let grouped = group_mode != GroupMode::None;
            let history_path = PathBuf::from(settings_key.clone());
            let qptr = QPointer::from(&*self);
            let apply_result = qmetaobject::queued_callback(move |result: LoadResult| {
                if let Some(model) = qptr.as_pinned() {
                    model.borrow_mut().apply_load_result(result);
                }
            });
            thread::spawn(move || {
                let result = list_archive_loaded_entries(&archive_path, &inner_path, group_mode)
                    .map(|mut entries| {
                        sort_loaded_entries(
                            &mut entries,
                            &sort_field_name,
                            sort_descending,
                            grouped,
                        );
                        entries
                    })
                    .map_err(|error| {
                        format!("Failed to list archive {}: {error}", archive_path.display())
                    });

                apply_result(LoadResult {
                    generation,
                    path: settings_key,
                    history_path,
                    record_history,
                    group_mode,
                    result,
                });
            });
        }

        fn next_load_generation(&self) -> usize {
            self.load_generation.fetch_add(1, Ordering::SeqCst) + 1
        }

        fn clear_entries_for_load(&mut self) {
            self.begin_reset_model();
            self.entries.clear();
            self.grouped_entries_json = QString::default();
            self.grouped_entries_json_changed();
            self.end_reset_model();
            self.set_folder_info();
            self.error_message = QString::default();
            self.error_message_changed();
        }

        fn apply_load_result(&mut self, result: LoadResult) {
            if self.load_generation.load(Ordering::SeqCst) != result.generation {
                return;
            }

            match result.result {
                Ok(entries) => {
                    self.begin_reset_model();
                    let mut items: Vec<FileItem> =
                        entries.into_iter().map(FileItem::from_loaded).collect();
                    let mut previous_group = String::new();
                    for item in &mut items {
                        let current_group = item.group_label.to_string();
                        item.group_start =
                            !current_group.is_empty() && current_group != previous_group;
                        previous_group = current_group;
                    }
                    self.grouped_entries_json = if result.group_mode == GroupMode::None {
                        QString::default()
                    } else {
                        QString::from(build_grouped_entries_json(&items))
                    };
                    self.grouped_entries_json_changed();
                    self.entries = items;
                    self.end_reset_model();
                    self.current_path = QString::from(result.path);
                    self.current_path_changed();
                    self.set_folder_info();
                    self.error_message = QString::default();
                    self.error_message_changed();
                    if result.record_history {
                        self.push_history(result.history_path);
                    } else {
                        self.update_history_flags();
                    }
                }
                Err(error) => {
                    self.error_message = QString::from(error);
                    self.error_message_changed();
                    if result.record_history {
                        self.push_history(result.history_path);
                    } else {
                        self.update_history_flags();
                    }
                }
            }
        }

        fn push_history(&mut self, path: PathBuf) {
            if self.history.get(self.history_index) == Some(&path) {
                self.update_history_flags();
                return;
            }

            if self.history_index + 1 < self.history.len() {
                self.history.truncate(self.history_index + 1);
            }

            self.history.push(path);
            self.history_index = self.history.len().saturating_sub(1);
            self.update_history_flags();
        }

        fn update_history_flags(&mut self) {
            let can_go_back = self.history_index > 0;
            let can_go_forward = self.history_index + 1 < self.history.len();
            if self.can_go_back != can_go_back {
                self.can_go_back = can_go_back;
                self.can_go_back_changed();
            }
            if self.can_go_forward != can_go_forward {
                self.can_go_forward = can_go_forward;
                self.can_go_forward_changed();
            }
        }

        fn reload_current_directory(&mut self) {
            let current_path = normalize_path_input(&self.current_path.to_string());
            self.load_path_without_history(current_path);
        }

        fn terminal_working_directory(&self) -> PathBuf {
            let current_path = self.current_path.to_string();
            if is_virtual_location(&current_path) {
                return env::var_os("HOME")
                    .map(PathBuf::from)
                    .unwrap_or_else(|| PathBuf::from("/"));
            }
            if let Some((archive_path, _)) = parse_archive_uri(&current_path) {
                return archive_path
                    .parent()
                    .map(Path::to_path_buf)
                    .unwrap_or_else(|| {
                        env::var_os("HOME")
                            .map(PathBuf::from)
                            .unwrap_or_else(|| PathBuf::from("/"))
                    });
            }

            let path = PathBuf::from(&current_path);
            if path.is_dir() {
                path
            } else {
                path.parent()
                    .map(Path::to_path_buf)
                    .unwrap_or_else(|| PathBuf::from("/"))
            }
        }

        fn set_folder_info(&mut self) {
            let current_path = self.current_path.to_string();
            let label = if current_path == FAVORITES_URI {
                "Favourites".to_string()
            } else if current_path == DEVICES_URI {
                "Devices".to_string()
            } else {
                if let Some((archive_path, inner_path)) = parse_archive_uri(&current_path) {
                    if inner_path.is_empty() {
                        archive_path
                            .file_name()
                            .and_then(|value| value.to_str())
                            .map(str::to_string)
                            .unwrap_or_else(|| archive_path.display().to_string())
                    } else {
                        inner_path
                            .split('/')
                            .rfind(|part| !part.is_empty())
                            .map(str::to_string)
                            .unwrap_or_else(|| archive_path.display().to_string())
                    }
                } else {
                    Path::new(&current_path)
                        .file_name()
                        .and_then(|value| value.to_str())
                        .map(str::to_string)
                        .filter(|value| !value.is_empty())
                        .unwrap_or_else(|| current_path.clone())
                }
            };
            self.selected_info_name = QString::from(label);
            self.selected_info_kind =
                QString::from(if parse_archive_uri(&current_path).is_some() {
                    "Archive Folder"
                } else if is_virtual_location(&current_path) {
                    "Virtual Folder"
                } else {
                    "Folder"
                });
            self.selected_info_size = QString::from(format!("{} items", self.entries.len()));
            self.selected_info_modified = QString::from("-");
            self.selected_info_modified_ms = 0.0;
            self.selected_info_path = QString::from(current_path);
            self.selected_info_thumbnail_url = QString::default();
            self.selected_info_is_directory = true;
            self.selected_info_changed();
        }

        fn update_selected_info(&mut self, path: &str) {
            if path.is_empty() {
                self.set_folder_info();
                return;
            }

            if let Some(item) = self
                .entries
                .iter()
                .find(|item| item.path.to_string() == path)
            {
                self.selected_info_name = item.name.clone();
                self.selected_info_kind = item.kind.clone();
                self.selected_info_size = item.size_text.clone();
                self.selected_info_modified = item.modified_text.clone();
                self.selected_info_modified_ms = item.modified_ms as f64;
                self.selected_info_path = item.path.clone();
                self.selected_info_thumbnail_url = item.thumbnail_url.clone();
                self.selected_info_is_directory = item.is_directory;
                self.selected_info_changed();
                return;
            }

            self.selected_info_name = QString::from(path);
            self.selected_info_kind = QString::from("Item");
            self.selected_info_size = QString::from("-");
            self.selected_info_modified = QString::from("-");
            self.selected_info_modified_ms = 0.0;
            self.selected_info_path = QString::from(path);
            self.selected_info_thumbnail_url = QString::default();
            self.selected_info_is_directory = false;
            self.selected_info_changed();
        }
    }

    impl QAbstractListModel for FilesModel {
        fn row_count(&self) -> i32 {
            self.entries.len() as i32
        }

        fn data(&self, index: QModelIndex, role: i32) -> QVariant {
            let row = index.row();
            if row < 0 || (row as usize) >= self.entries.len() {
                return QVariant::default();
            }

            let item = &self.entries[row as usize];
            match role {
                ROLE_NAME => item.name.clone().into(),
                ROLE_PATH => item.path.clone().into(),
                ROLE_KIND => item.kind.clone().into(),
                ROLE_SIZE_TEXT => item.size_text.clone().into(),
                ROLE_MODIFIED_TEXT => item.modified_text.clone().into(),
                ROLE_HIDDEN => item.hidden.into(),
                ROLE_IS_DIRECTORY => item.is_directory.into(),
                ROLE_MODIFIED_MS => (item.modified_ms as f64).into(),
                ROLE_THUMBNAIL_URL => item.thumbnail_url.clone().into(),
                ROLE_GROUP_LABEL => item.group_label.clone().into(),
                ROLE_GROUP_START => item.group_start.into(),
                _ => QVariant::default(),
            }
        }

        fn role_names(&self) -> HashMap<i32, QByteArray> {
            HashMap::from([
                (ROLE_NAME, QByteArray::from("name")),
                (ROLE_PATH, QByteArray::from("path")),
                (ROLE_KIND, QByteArray::from("kind")),
                (ROLE_SIZE_TEXT, QByteArray::from("sizeText")),
                (ROLE_MODIFIED_TEXT, QByteArray::from("modifiedText")),
                (ROLE_HIDDEN, QByteArray::from("hidden")),
                (ROLE_IS_DIRECTORY, QByteArray::from("isDirectory")),
                (ROLE_MODIFIED_MS, QByteArray::from("modifiedMs")),
                (ROLE_THUMBNAIL_URL, QByteArray::from("thumbnailUrl")),
                (ROLE_GROUP_LABEL, QByteArray::from("groupLabel")),
                (ROLE_GROUP_START, QByteArray::from("groupStart")),
            ])
        }
    }

    impl FileItem {
        fn from_loaded(value: LoadedEntry) -> Self {
            Self {
                name: QString::from(value.name),
                path: QString::from(value.path),
                kind: QString::from(match value.kind {
                    EntryKind::Directory => "Folder",
                    EntryKind::File => "File",
                    EntryKind::Symlink => "Symlink",
                    EntryKind::Other => "Other",
                }),
                size_text: QString::from(value.size_text),
                modified_text: QString::from(value.modified_text),
                modified_ms: value.modified_ms,
                thumbnail_url: QString::from(value.thumbnail_url),
                group_label: QString::from(value.group_label),
                group_start: false,
                hidden: value.hidden,
                is_directory: value.is_directory,
            }
        }

        fn virtual_shortcut(
            name: String,
            target_path: String,
            details: String,
            kind: &str,
            group_label: String,
        ) -> Self {
            Self {
                name: QString::from(name),
                path: QString::from(target_path),
                kind: QString::from(kind),
                size_text: QString::from(details),
                modified_text: QString::from("-"),
                modified_ms: 0,
                thumbnail_url: QString::default(),
                group_label: QString::from(group_label),
                group_start: false,
                hidden: false,
                is_directory: true,
            }
        }
    }

    impl LoadedEntry {
        fn from_entry(
            value: FileEntry,
            group_mode: GroupMode,
            thumbnail_jobs: &Arc<AtomicUsize>,
            queued_thumbnails: &Arc<Mutex<HashSet<PathBuf>>>,
        ) -> Self {
            let thumbnail_url = thumbnail_url_for_entry(&value, thumbnail_jobs, queued_thumbnails);
            Self {
                name: value.name.clone(),
                path: value.path.display().to_string(),
                kind: value.kind,
                size_text: format_size(value.size_bytes, value.kind),
                modified_text: format_modified(value.modified),
                modified_ms: modified_millis(value.modified),
                thumbnail_url,
                group_label: group_label(&value, group_mode),
                hidden: value.hidden,
                is_directory: value.kind == EntryKind::Directory,
            }
        }
    }

    #[derive(Serialize)]
    struct GroupJson {
        label: String,
        items: Vec<GroupItemJson>,
    }

    #[derive(Serialize)]
    struct GroupItemJson {
        name: String,
        path: String,
        kind: String,
        size_text: String,
        modified_text: String,
        modified_ms: u64,
        thumbnail_url: String,
        is_directory: bool,
    }

    #[derive(QObject, Default)]
    struct DevicesModel {
        #[qt_base_class = "QAbstractListModel"]
        base: qmetaobject::QObjectCppWrapper,
        entries: Vec<DeviceItem>,
        error_message: qt_property!(QString; NOTIFY error_message_changed),
        error_message_changed: qt_signal!(),
        refresh: qt_method!(
            fn refresh(&mut self) {
                self.reload();
            }
        ),
    }

    impl DevicesModel {
        fn reload(&mut self) {
            match list_devices() {
                Ok(entries) => {
                    self.begin_reset_model();
                    self.entries = entries;
                    self.end_reset_model();
                    self.error_message = QString::default();
                    self.error_message_changed();
                }
                Err(error) => {
                    self.error_message = QString::from(format!("Failed to load devices: {error}"));
                    self.error_message_changed();
                }
            }
        }
    }

    impl QAbstractListModel for DevicesModel {
        fn row_count(&self) -> i32 {
            self.entries.len() as i32
        }

        fn data(&self, index: QModelIndex, role: i32) -> QVariant {
            let row = index.row();
            if row < 0 || (row as usize) >= self.entries.len() {
                return QVariant::default();
            }

            let item = &self.entries[row as usize];
            match role {
                ROLE_NAME => item.label.clone().into(),
                ROLE_PATH => item.mount_path.clone().into(),
                ROLE_SIZE_TEXT => item.details.clone().into(),
                ROLE_IS_DIRECTORY => item.mounted.into(),
                ROLE_DEVICE_USAGE => item.usage_percent.into(),
                _ => QVariant::default(),
            }
        }

        fn role_names(&self) -> HashMap<i32, QByteArray> {
            HashMap::from([
                (ROLE_NAME, QByteArray::from("label")),
                (ROLE_PATH, QByteArray::from("mountPath")),
                (ROLE_SIZE_TEXT, QByteArray::from("details")),
                (ROLE_IS_DIRECTORY, QByteArray::from("mounted")),
                (ROLE_DEVICE_USAGE, QByteArray::from("usagePercent")),
            ])
        }
    }

    fn format_size(size_bytes: u64, kind: EntryKind) -> String {
        if kind == EntryKind::Directory {
            return "-".to_string();
        }

        const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
        let mut value = size_bytes as f64;
        let mut unit = 0_usize;

        while value >= 1024.0 && unit < UNITS.len() - 1 {
            value /= 1024.0;
            unit += 1;
        }

        if unit == 0 {
            format!("{size_bytes} {}", UNITS[unit])
        } else {
            format!("{value:.1} {}", UNITS[unit])
        }
    }

    fn format_modified(modified: Option<std::time::SystemTime>) -> String {
        if modified.is_some() {
            String::new()
        } else {
            "-".to_string()
        }
    }

    fn modified_millis(modified: Option<std::time::SystemTime>) -> u64 {
        let Some(modified) = modified else {
            return 0;
        };

        match modified.duration_since(std::time::UNIX_EPOCH) {
            Ok(duration) => duration.as_millis().min(u128::from(u64::MAX)) as u64,
            Err(_) => 0,
        }
    }

    fn build_grouped_entries_json(items: &[FileItem]) -> String {
        let mut groups: Vec<GroupJson> = Vec::new();

        for item in items {
            let label = item.group_label.to_string();
            if label.is_empty() {
                continue;
            }

            let needs_new_group = groups
                .last()
                .map(|group| group.label != label)
                .unwrap_or(true);

            if needs_new_group {
                groups.push(GroupJson {
                    label: label.clone(),
                    items: Vec::new(),
                });
            }

            if let Some(group) = groups.last_mut() {
                group.items.push(GroupItemJson {
                    name: item.name.to_string(),
                    path: item.path.to_string(),
                    kind: item.kind.to_string(),
                    size_text: item.size_text.to_string(),
                    modified_text: item.modified_text.to_string(),
                    modified_ms: item.modified_ms,
                    thumbnail_url: item.thumbnail_url.to_string(),
                    is_directory: item.is_directory,
                });
            }
        }

        serde_json::to_string(&groups).unwrap_or_else(|_| "[]".to_string())
    }

    fn list_virtual_entries(location: &str, grouped: bool) -> io::Result<Vec<FileItem>> {
        match location {
            FAVORITES_URI => Ok(favorite_entries(grouped)),
            DEVICES_URI => device_entries(grouped),
            _ => Err(io::Error::new(
                io::ErrorKind::NotFound,
                "unknown virtual folder",
            )),
        }
    }

    fn list_archive_loaded_entries(
        archive_path: &Path,
        inner_path: &str,
        group_mode: GroupMode,
    ) -> io::Result<Vec<LoadedEntry>> {
        let mut seen = HashSet::new();
        let mut items = Vec::new();
        for entry in cached_archive_entries(archive_path)? {
            if let Some(item) =
                archive_raw_entry_to_loaded(archive_path, inner_path, &entry, group_mode)
            {
                let key = item.path.clone();
                if seen.insert(key) {
                    items.push(item);
                }
            }
        }

        Ok(items)
    }

    fn cached_archive_entries(archive_path: &Path) -> io::Result<Vec<ArchiveRawEntry>> {
        static ARCHIVE_CACHE: OnceLock<Mutex<HashMap<ArchiveCacheKey, Vec<ArchiveRawEntry>>>> =
            OnceLock::new();

        let key = archive_cache_key(archive_path)?;
        let cache = ARCHIVE_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
        {
            let cache = cache.lock().expect("archive cache poisoned");
            if let Some(entries) = cache.get(&key) {
                return Ok(entries.clone());
            }
        }

        let entries = read_archive_entries(archive_path)?;
        let mut cache = cache.lock().expect("archive cache poisoned");
        cache.insert(key, entries.clone());
        Ok(entries)
    }

    fn archive_cache_key(archive_path: &Path) -> io::Result<ArchiveCacheKey> {
        let metadata = fs::metadata(archive_path)?;
        let modified = metadata
            .modified()
            .ok()
            .and_then(|value| value.duration_since(UNIX_EPOCH).ok())
            .unwrap_or_default();

        Ok(ArchiveCacheKey {
            path: archive_path.to_path_buf(),
            size_bytes: metadata.len(),
            modified_secs: modified.as_secs(),
            modified_nanos: modified.subsec_nanos(),
        })
    }

    fn read_archive_entries(archive_path: &Path) -> io::Result<Vec<ArchiveRawEntry>> {
        let output = Command::new("7z")
            .args(["l", "-slt", archive_path.to_string_lossy().as_ref()])
            .output()?;

        if !output.status.success() {
            return Err(io::Error::other(format!(
                "7z list exited with status {}",
                output.status
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut entries = Vec::new();
        let mut current: HashMap<String, String> = HashMap::new();

        for line in stdout.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                if let Some(entry) = archive_raw_entry_from_block(&current) {
                    entries.push(entry);
                }
                current.clear();
                continue;
            }

            if trimmed == "----------" || trimmed == "--" || trimmed.starts_with("Path = /") {
                continue;
            }

            if let Some((key, value)) = trimmed.split_once(" = ") {
                current.insert(key.to_string(), value.to_string());
            }
        }

        if let Some(entry) = archive_raw_entry_from_block(&current) {
            entries.push(entry);
        }

        Ok(entries)
    }

    fn archive_raw_entry_from_block(block: &HashMap<String, String>) -> Option<ArchiveRawEntry> {
        let path = block.get("Path")?;
        Some(ArchiveRawEntry {
            path: path.to_string(),
            folder: block.get("Folder").map(String::as_str).unwrap_or("-") == "+",
            size_bytes: block
                .get("Size")
                .and_then(|value| value.parse::<u64>().ok())
                .unwrap_or(0),
            modified: block
                .get("Modified")
                .cloned()
                .unwrap_or_else(|| "-".to_string()),
        })
    }

    fn archive_raw_entry_to_loaded(
        archive_path: &Path,
        inner_path: &str,
        entry: &ArchiveRawEntry,
        group_mode: GroupMode,
    ) -> Option<LoadedEntry> {
        let normalized_inner = inner_path.trim_matches('/');
        let prefix = if normalized_inner.is_empty() {
            String::new()
        } else {
            format!("{normalized_inner}/")
        };

        if !prefix.is_empty() && !entry.path.starts_with(&prefix) {
            return None;
        }

        let remaining = if prefix.is_empty() {
            entry.path.as_str()
        } else {
            &entry.path[prefix.len()..]
        };
        if remaining.is_empty() {
            return None;
        }

        let immediate = remaining.split('/').next().unwrap_or(remaining);
        let is_directory = entry.folder || remaining.contains('/');
        let child_inner = if normalized_inner.is_empty() {
            immediate.to_string()
        } else {
            format!("{normalized_inner}/{immediate}")
        };

        let virtual_path = archive_uri(archive_path, &child_inner);
        let kind = if is_directory {
            EntryKind::Directory
        } else {
            EntryKind::File
        };
        let file_entry = FileEntry {
            name: immediate.to_string(),
            path: PathBuf::from(&virtual_path),
            kind,
            size_bytes: entry.size_bytes,
            modified: None,
            hidden: false,
        };

        Some(LoadedEntry {
            name: immediate.to_string(),
            path: virtual_path,
            kind,
            size_text: format_size(entry.size_bytes, kind),
            modified_text: entry.modified.clone(),
            modified_ms: 0,
            thumbnail_url: String::new(),
            group_label: group_label(&file_entry, group_mode),
            hidden: false,
            is_directory,
        })
    }

    fn favorite_entries(grouped: bool) -> Vec<FileItem> {
        let entries = favorite_locations();
        entries
            .into_iter()
            .filter(|(_, path)| Path::new(path).exists())
            .map(|(label, path)| {
                FileItem::virtual_shortcut(
                    label,
                    path.clone(),
                    "Shortcut".to_string(),
                    "Shortcut",
                    if grouped {
                        "Shortcuts".to_string()
                    } else {
                        String::new()
                    },
                )
            })
            .collect()
    }

    fn device_entries(grouped: bool) -> io::Result<Vec<FileItem>> {
        Ok(list_devices()?
            .into_iter()
            .filter(|device| device.mounted && !device.mount_path.to_string().is_empty())
            .map(|device| {
                FileItem::virtual_shortcut(
                    device.label.to_string(),
                    device.mount_path.to_string(),
                    device.details.to_string(),
                    "Device",
                    if grouped {
                        "Mounted devices".to_string()
                    } else {
                        String::new()
                    },
                )
            })
            .collect())
    }

    fn favorite_locations() -> Vec<(String, String)> {
        let home = PlatformPaths::writable_location("HOME")
            .or_else(|| env::var("HOME").ok())
            .unwrap_or_else(|| "/".to_string());
        vec![
            ("Home".to_string(), home.clone()),
            ("Desktop".to_string(), format!("{home}/Desktop")),
            ("Documents".to_string(), format!("{home}/Documents")),
            ("Downloads".to_string(), format!("{home}/Downloads")),
            ("sysApps".to_string(), "/home/tamsynn/sysApps".to_string()),
        ]
    }

    fn sort_virtual_items(
        items: &mut [FileItem],
        sort_field_name: &str,
        descending: bool,
        grouped: bool,
    ) {
        items.sort_by(|left, right| {
            let group_order = if grouped {
                left.group_label
                    .to_string()
                    .cmp(&right.group_label.to_string())
            } else {
                std::cmp::Ordering::Equal
            };
            if group_order != std::cmp::Ordering::Equal {
                return group_order;
            }

            let ordering = match sort_field_name {
                "Type" => left.kind.to_string().cmp(&right.kind.to_string()),
                "Size" => left.size_text.to_string().cmp(&right.size_text.to_string()),
                "Modified" => left.modified_ms.cmp(&right.modified_ms),
                _ => left
                    .name
                    .to_string()
                    .to_lowercase()
                    .cmp(&right.name.to_string().to_lowercase()),
            };

            if descending {
                ordering.reverse()
            } else {
                ordering
            }
        });
    }

    fn sort_loaded_entries(
        items: &mut [LoadedEntry],
        sort_field_name: &str,
        descending: bool,
        grouped: bool,
    ) {
        items.sort_by(|left, right| {
            let group_order = if grouped {
                left.group_label.cmp(&right.group_label)
            } else {
                std::cmp::Ordering::Equal
            };
            if group_order != std::cmp::Ordering::Equal {
                return group_order;
            }

            match (left.is_directory, right.is_directory) {
                (true, false) => return std::cmp::Ordering::Less,
                (false, true) => return std::cmp::Ordering::Greater,
                _ => {}
            }

            let ordering = match sort_field_name {
                "Type" => left.kind.cmp(&right.kind),
                "Size" => left.size_text.cmp(&right.size_text),
                "Modified" => left.modified_ms.cmp(&right.modified_ms),
                _ => left.name.to_lowercase().cmp(&right.name.to_lowercase()),
            };

            if descending {
                ordering.reverse()
            } else {
                ordering
            }
        });
    }

    fn is_virtual_location(path: &str) -> bool {
        path.starts_with("virtual://")
    }

    fn is_read_only_location(path: &str) -> bool {
        is_virtual_location(path) || parse_archive_uri(path).is_some()
    }

    fn read_folder_settings(folder_key: &str) -> io::Result<FolderSettings> {
        let store = read_folder_settings_store()?;
        Ok(store.folders.get(folder_key).cloned().unwrap_or_default())
    }

    fn write_folder_settings(folder_key: &str, settings: FolderSettings) -> io::Result<()> {
        let mut store = read_folder_settings_store().unwrap_or_default();
        store.folders.insert(folder_key.to_string(), settings);

        let path = folder_settings_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let contents = serde_json::to_string_pretty(&store)
            .map_err(|error| io::Error::other(format!("failed to encode settings: {error}")))?;
        fs::write(path, contents)
    }

    fn read_folder_settings_store() -> io::Result<FolderSettingsStore> {
        let path = folder_settings_path();
        if !path.exists() {
            return Ok(FolderSettingsStore::default());
        }

        let contents = fs::read_to_string(path)?;
        serde_json::from_str(&contents)
            .map_err(|error| io::Error::other(format!("failed to parse folder settings: {error}")))
    }

    fn folder_settings_path() -> PathBuf {
        config_root().join("files/folder-settings.json")
    }

    fn config_root() -> PathBuf {
        env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
            .unwrap_or_else(|| env::temp_dir().join("sysApps-config"))
            .join("sysApps")
    }

    fn parse_archive_uri(uri: &str) -> Option<(PathBuf, String)> {
        let rest = uri.strip_prefix(ARCHIVE_URI_PREFIX)?;
        let (archive_path, inner_path) = rest
            .split_once("!/")
            .map_or((rest, ""), |(archive, inner)| (archive, inner));
        Some((PathBuf::from(archive_path), inner_path.to_string()))
    }

    fn archive_uri(archive_path: &Path, inner_path: &str) -> String {
        if inner_path.is_empty() {
            format!("{ARCHIVE_URI_PREFIX}{}!/", archive_path.display())
        } else {
            format!(
                "{ARCHIVE_URI_PREFIX}{}!/{}",
                archive_path.display(),
                inner_path.trim_matches('/')
            )
        }
    }

    fn archive_parent_path(inner_path: &str) -> String {
        let trimmed = inner_path.trim_matches('/');
        if let Some((parent, _)) = trimmed.rsplit_once('/') {
            parent.to_string()
        } else {
            String::new()
        }
    }

    struct PlatformPaths;

    impl PlatformPaths {
        fn writable_location(name: &str) -> Option<String> {
            env::var(name).ok()
        }
    }

    fn open_with_default(path: &Path) -> std::io::Result<()> {
        let status = Command::new("xdg-open").arg(path).status()?;
        if status.success() {
            Ok(())
        } else {
            Err(std::io::Error::other(format!(
                "xdg-open exited with status {status}"
            )))
        }
    }

    fn open_in_image_viewer(target: &str) -> io::Result<()> {
        if let Some(configured_path) = env::var_os("SYSAPPS_IMAGE_VIEWER_BIN") {
            return spawn_viewer_process(PathBuf::from(configured_path), target);
        }

        if let Ok(current_exe) = env::current_exe() {
            let sibling_binary = current_exe.with_file_name("image-viewer-app");
            if sibling_binary.is_file() {
                return spawn_viewer_process(sibling_binary, target);
            }
        }

        let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .map(Path::to_path_buf)
            .ok_or_else(|| {
                io::Error::new(io::ErrorKind::NotFound, "failed to locate workspace root")
            })?;
        let cargo_manifest = workspace_root.join("Cargo.toml");

        if cargo_manifest.is_file() {
            let status = Command::new("cargo")
                .args(["run", "-p", "image-viewer-app", "--features", "qml", "--"])
                .arg(target)
                .current_dir(&workspace_root)
                .spawn();

            return status.map(|_| ());
        }

        Err(io::Error::new(
            io::ErrorKind::NotFound,
            "image viewer binary was not found",
        ))
    }

    fn spawn_viewer_process(program: PathBuf, target: &str) -> io::Result<()> {
        Command::new(program).arg(target).spawn().map(|_| ())
    }

    fn qml_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("qml/Main.qml")
    }

    fn list_devices() -> io::Result<Vec<DeviceItem>> {
        let output = Command::new("lsblk")
            .args([
                "-P",
                "-o",
                "NAME,LABEL,MOUNTPOINT,SIZE,TYPE,RM,HOTPLUG,FSUSE%",
            ])
            .output()?;

        if !output.status.success() {
            return Err(io::Error::other(format!(
                "lsblk exited with status {}",
                output.status
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut devices = Vec::new();

        for line in stdout.lines() {
            let fields = parse_lsblk_pairs(line);
            let type_name = fields.get("TYPE").map(String::as_str).unwrap_or("");
            let mount_path = fields.get("MOUNTPOINT").cloned().unwrap_or_default();
            if should_hide_device_mount(&mount_path) {
                continue;
            }

            let is_removable = matches!(fields.get("RM").map(String::as_str), Some("1"))
                || matches!(fields.get("HOTPLUG").map(String::as_str), Some("1"));

            let should_show = !mount_path.is_empty() || is_removable || type_name == "rom";
            if !should_show || !(type_name == "disk" || type_name == "part" || type_name == "rom") {
                continue;
            }

            let name = fields
                .get("NAME")
                .cloned()
                .unwrap_or_else(|| "device".to_string());
            let label = fields.get("LABEL").cloned().unwrap_or_default();
            let size = fields.get("SIZE").cloned().unwrap_or_default();
            let display_label = if !label.is_empty() {
                label
            } else if !mount_path.is_empty() {
                format!("{name} ({mount_path})")
            } else {
                name.clone()
            };

            let details = size.clone();
            let usage_percent = fields
                .get("FSUSE%")
                .map(String::as_str)
                .map(parse_usage_percent)
                .unwrap_or(0.0);

            devices.push(DeviceItem {
                label: QString::from(display_label),
                mount_path: QString::from(mount_path),
                details: QString::from(details),
                usage_percent,
                mounted: !fields
                    .get("MOUNTPOINT")
                    .cloned()
                    .unwrap_or_default()
                    .is_empty(),
            });
        }

        Ok(devices)
    }

    fn should_hide_device_mount(mount_path: &str) -> bool {
        mount_path == "/boot/efi"
    }

    fn parse_lsblk_pairs(line: &str) -> HashMap<String, String> {
        let mut fields = HashMap::new();
        for token in line.split("\" ") {
            let token = token.trim();
            if token.is_empty() {
                continue;
            }
            if let Some((key, value)) = token.split_once("=\"") {
                fields.insert(key.to_string(), value.trim_end_matches('"').to_string());
            }
        }
        fields
    }

    fn parse_usage_percent(value: &str) -> f64 {
        value
            .trim()
            .trim_end_matches('%')
            .parse::<f64>()
            .map(|percent| percent.clamp(0.0, 100.0))
            .unwrap_or(0.0)
    }

    fn run_shell_command(command: &str, cwd: &Path) -> io::Result<String> {
        let output = Command::new("bash")
            .arg("-lc")
            .arg(command)
            .current_dir(cwd)
            .output()?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let mut combined = String::new();

        if !stdout.trim_end().is_empty() {
            combined.push_str(stdout.trim_end());
        }
        if !stderr.trim_end().is_empty() {
            if !combined.is_empty() {
                combined.push('\n');
            }
            combined.push_str(stderr.trim_end());
        }
        if !output.status.success() {
            if !combined.is_empty() {
                combined.push('\n');
            }
            let _ = write!(&mut combined, "Exit status: {}", output.status);
        }

        Ok(combined)
    }

    fn open_konsole(cwd: &Path) -> io::Result<()> {
        Command::new("konsole")
            .arg("--workdir")
            .arg(cwd)
            .spawn()
            .map(|_| ())
    }

    fn open_noxcmd(cwd: &Path) -> io::Result<()> {
        if let Some(configured_path) = env::var_os("SYSAPPS_NOXCMD_BIN") {
            return Command::new(configured_path).arg(cwd).spawn().map(|_| ());
        }

        let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .map(Path::to_path_buf)
            .ok_or_else(|| {
                io::Error::new(io::ErrorKind::NotFound, "failed to locate workspace root")
            })?;

        if workspace_root.join("Cargo.toml").is_file() {
            return Command::new("cargo")
                .args(["run", "-p", "noxcmd-app", "--features", "qml", "--"])
                .arg(cwd)
                .current_dir(workspace_root)
                .spawn()
                .map(|_| ());
        }

        Command::new("noxcmd-app").arg(cwd).spawn().map(|_| ())
    }

    fn thumbnail_url_for_entry(
        entry: &FileEntry,
        thumbnail_jobs: &Arc<AtomicUsize>,
        queued_thumbnails: &Arc<Mutex<HashSet<PathBuf>>>,
    ) -> String {
        if entry.kind != EntryKind::File {
            return String::new();
        }

        if is_image_extension(&entry.path) {
            return file_url_for_path(&entry.path);
        }

        if is_blender_thumbnail_extension(&entry.path) {
            let cache_file = blender_thumbnail_cache_path(&entry.path, entry.modified);
            if cache_file.exists() {
                return file_url_for_path(&cache_file);
            }

            queue_blender_thumbnail(
                entry.path.clone(),
                entry.modified,
                Arc::clone(thumbnail_jobs),
                Arc::clone(queued_thumbnails),
            );
            return String::new();
        }

        String::new()
    }

    fn is_image_extension(path: &Path) -> bool {
        matches!(
            path.extension().and_then(|ext| ext.to_str()).map(|ext| ext.to_ascii_lowercase()),
            Some(ext) if matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp" | "svg")
        )
    }

    fn is_video_extension(path: &Path) -> bool {
        matches!(
            path.extension().and_then(|ext| ext.to_str()).map(|ext| ext.to_ascii_lowercase()),
            Some(ext) if matches!(ext.as_str(), "mp4" | "mkv" | "webm" | "mov" | "avi" | "m4v" | "ogv")
        )
    }

    fn is_supported_media(path: &Path) -> bool {
        is_image_extension(path) || is_video_extension(path)
    }

    fn is_archive_extension(path: &Path) -> bool {
        matches!(
            path.extension().and_then(|ext| ext.to_str()).map(|ext| ext.to_ascii_lowercase()),
            Some(ext) if matches!(
                ext.as_str(),
                "zip" | "rar" | "7z" | "tar" | "gz" | "bz2" | "xz" | "tgz" | "tbz2" | "txz"
            )
        )
    }

    fn is_blender_thumbnail_extension(path: &Path) -> bool {
        matches!(
            path.extension().and_then(|ext| ext.to_str()).map(|ext| ext.to_ascii_lowercase()),
            Some(ext) if matches!(
                ext.as_str(),
                "stl" | "obj" | "ply" | "fbx" | "glb" | "gltf" | "abc" | "x3d" | "usd" | "usda" | "usdc" | "usdz"
            )
        )
    }

    fn generate_blender_thumbnail(path: &Path, modified: Option<SystemTime>) -> io::Result<String> {
        let cache_file = blender_thumbnail_cache_path(path, modified);
        if cache_file.exists() {
            return Ok(file_url_for_path(&cache_file));
        }

        if let Some(parent) = cache_file.parent() {
            fs::create_dir_all(parent)?;
        }

        let script_path = blender_thumbnail_script_path();
        if !script_path.exists() {
            fs::write(&script_path, BLENDER_THUMBNAIL_SCRIPT)?;
        }

        let status = Command::new("blender")
            .args([
                "-b",
                "--factory-startup",
                "--python",
                script_path.to_string_lossy().as_ref(),
                "--",
                path.to_string_lossy().as_ref(),
                cache_file.to_string_lossy().as_ref(),
            ])
            .status()?;

        if !status.success() || !cache_file.exists() {
            return Err(io::Error::other(format!(
                "blender thumbnail generation failed for {}",
                path.display()
            )));
        }

        Ok(file_url_for_path(&cache_file))
    }

    fn queue_blender_thumbnail(
        path: PathBuf,
        modified: Option<SystemTime>,
        thumbnail_jobs: Arc<AtomicUsize>,
        queued_thumbnails: Arc<Mutex<HashSet<PathBuf>>>,
    ) {
        let cache_file = blender_thumbnail_cache_path(&path, modified);

        {
            let mut queued = queued_thumbnails.lock().expect("thumbnail queue poisoned");
            if queued.contains(&cache_file) {
                return;
            }
            queued.insert(cache_file.clone());
        }

        thumbnail_jobs.fetch_add(1, Ordering::SeqCst);
        thread::spawn(move || {
            let _ = generate_blender_thumbnail(&path, modified);
            thumbnail_jobs.fetch_sub(1, Ordering::SeqCst);
            let mut queued = queued_thumbnails.lock().expect("thumbnail queue poisoned");
            queued.remove(&cache_file);
        });
    }

    fn blender_thumbnail_cache_path(path: &Path, modified: Option<SystemTime>) -> PathBuf {
        let mut hasher = DefaultHasher::new();
        path.hash(&mut hasher);
        modified_millis(modified).hash(&mut hasher);
        if let Ok(metadata) = fs::metadata(path) {
            metadata.len().hash(&mut hasher);
        }
        let digest = format!("{:016x}", hasher.finish());
        thumbnail_cache_root()
            .join("models")
            .join(format!("{digest}.png"))
    }

    fn thumbnail_cache_root() -> PathBuf {
        env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join(".cache/sysApps/thumbnails")
    }

    fn blender_thumbnail_script_path() -> PathBuf {
        env::temp_dir().join("sysapps_blender_thumbnail.py")
    }

    fn file_url_for_path(path: &Path) -> String {
        format!("file://{}", encode_trash_path(path))
    }

    fn open_archive_member(archive_path: &Path, inner_path: &str) -> io::Result<()> {
        let destination = archive_member_cache_path(archive_path, inner_path);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }

        if !destination.exists() {
            let output = Command::new("7z")
                .args([
                    "x",
                    "-so",
                    archive_path.to_string_lossy().as_ref(),
                    inner_path,
                ])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()?;

            if !output.status.success() {
                return Err(io::Error::other(
                    String::from_utf8_lossy(&output.stderr).to_string(),
                ));
            }

            fs::write(&destination, output.stdout)?;
        }

        open_with_default(&destination)
    }

    fn archive_member_cache_path(archive_path: &Path, inner_path: &str) -> PathBuf {
        let cache_root = env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join(".cache/sysApps/archive-preview");
        cache_root
            .join(cache_safe_archive_name(archive_path))
            .join(inner_path)
    }

    fn cache_safe_archive_name(path: &Path) -> String {
        let mut hasher = DefaultHasher::new();
        path.hash(&mut hasher);
        format!("{:016x}", hasher.finish())
    }

    const BLENDER_THUMBNAIL_SCRIPT: &str = r#"import bpy
import math
import mathutils
import sys
from pathlib import Path

argv = sys.argv
if "--" not in argv:
    raise SystemExit("missing args")
args = argv[argv.index("--") + 1:]
if len(args) != 2:
    raise SystemExit("expected input and output paths")

input_path = Path(args[0])
output_path = Path(args[1])
ext = input_path.suffix.lower()

def import_model(filepath, extension):
    if extension == ".stl":
        if hasattr(bpy.ops.wm, "stl_import"):
            bpy.ops.wm.stl_import(filepath=str(filepath))
        else:
            bpy.ops.import_mesh.stl(filepath=str(filepath))
    elif extension == ".obj":
        if hasattr(bpy.ops.wm, "obj_import"):
            bpy.ops.wm.obj_import(filepath=str(filepath))
        else:
            bpy.ops.import_scene.obj(filepath=str(filepath))
    elif extension == ".ply":
        if hasattr(bpy.ops.wm, "ply_import"):
            bpy.ops.wm.ply_import(filepath=str(filepath))
        else:
            bpy.ops.import_mesh.ply(filepath=str(filepath))
    elif extension == ".fbx":
        bpy.ops.import_scene.fbx(filepath=str(filepath))
    elif extension in (".glb", ".gltf"):
        bpy.ops.import_scene.gltf(filepath=str(filepath))
    elif extension == ".abc":
        bpy.ops.wm.alembic_import(filepath=str(filepath))
    elif extension == ".x3d":
        bpy.ops.import_scene.x3d(filepath=str(filepath))
    elif extension in (".usd", ".usda", ".usdc", ".usdz"):
        bpy.ops.wm.usd_import(filepath=str(filepath))
    else:
        raise RuntimeError(f"unsupported extension: {extension}")

bpy.ops.wm.read_factory_settings(use_empty=True)
scene = bpy.context.scene
scene.render.engine = "BLENDER_EEVEE"
scene.render.film_transparent = True
scene.render.image_settings.file_format = "PNG"
scene.render.resolution_x = 256
scene.render.resolution_y = 256
scene.render.filepath = str(output_path)

import_model(input_path, ext)

mesh_objects = [obj for obj in scene.objects if obj.type == "MESH"]
if not mesh_objects:
    raise RuntimeError("no mesh objects imported")

for obj in mesh_objects:
    obj.select_set(True)
    bpy.context.view_layer.objects.active = obj

bpy.ops.object.origin_set(type="ORIGIN_GEOMETRY", center="BOUNDS")

min_corner = mathutils.Vector((1e9, 1e9, 1e9))
max_corner = mathutils.Vector((-1e9, -1e9, -1e9))
for obj in mesh_objects:
    for corner in obj.bound_box:
        world_corner = obj.matrix_world @ mathutils.Vector(corner)
        min_corner.x = min(min_corner.x, world_corner.x)
        min_corner.y = min(min_corner.y, world_corner.y)
        min_corner.z = min(min_corner.z, world_corner.z)
        max_corner.x = max(max_corner.x, world_corner.x)
        max_corner.y = max(max_corner.y, world_corner.y)
        max_corner.z = max(max_corner.z, world_corner.z)

center = (min_corner + max_corner) / 2.0
size = max(max_corner.x - min_corner.x, max_corner.y - min_corner.y, max_corner.z - min_corner.z)
if size <= 0:
    size = 1.0

camera_data = bpy.data.cameras.new("ThumbnailCamera")
camera = bpy.data.objects.new("ThumbnailCamera", camera_data)
scene.collection.objects.link(camera)
scene.camera = camera

distance = size * 2.8
camera.location = center + mathutils.Vector((distance, -distance, distance * 0.75))
direction = center - camera.location
camera.rotation_euler = direction.to_track_quat("-Z", "Y").to_euler()

light_data = bpy.data.lights.new(name="ThumbnailSun", type="SUN")
light = bpy.data.objects.new(name="ThumbnailSun", object_data=light_data)
scene.collection.objects.link(light)
light.location = center + mathutils.Vector((distance * 0.5, -distance * 0.5, distance * 1.5))
light.rotation_euler = (math.radians(50), 0.0, math.radians(35))
light.data.energy = 3.0

bpy.ops.render.render(write_still=True)
"#;

    fn resolve_display_path(path: &Path) -> PathBuf {
        fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
    }

    fn create_directory_in_current(current_dir: &Path, name: &str) -> io::Result<()> {
        let clean_name = validate_new_name(name)?;
        fs::create_dir(current_dir.join(clean_name))
    }

    fn create_file_in_current(current_dir: &Path, name: &str) -> io::Result<()> {
        let clean_name = validate_new_name(name)?;
        fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(current_dir.join(clean_name))
            .map(|_| ())
    }

    fn rename_item(path: &Path, new_name: &str) -> io::Result<()> {
        let clean_name = validate_new_name(new_name)?;
        let parent = path.parent().ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "item has no parent directory")
        })?;
        fs::rename(path, parent.join(clean_name))
    }

    fn paste_clipboard_items(
        sources: &[PathBuf],
        destination: &Path,
        cut: bool,
        conflict_mode: PasteConflictMode,
        total_bytes: u64,
        mut on_progress: impl FnMut(u64, u64, f64, &Path),
    ) -> io::Result<()> {
        if !destination.is_dir() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "paste destination is not a folder",
            ));
        }

        let mut bytes_done = 0_u64;
        let started_at = Instant::now();
        for source in sources {
            if is_read_only_location(&source.to_string_lossy()) {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "read-only items cannot be pasted",
                ));
            }
            let elapsed = started_at.elapsed().as_secs_f64().max(0.001);
            on_progress(bytes_done, total_bytes, bytes_done as f64 / elapsed, source);
            let file_name = source.file_name().ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidInput, "item has no file name")
            })?;
            let initial_target = destination.join(file_name);
            if same_file_path(source, &initial_target) {
                continue;
            }

            let target = match conflict_mode {
                PasteConflictMode::Rename => unique_paste_destination(destination, file_name),
                PasteConflictMode::Overwrite => {
                    remove_existing_target(&initial_target)?;
                    initial_target
                }
            };
            if cut {
                move_item_with_progress(source, &target, &mut bytes_done, total_bytes, started_at, &mut on_progress)?;
            } else {
                copy_item_with_progress(source, &target, &mut bytes_done, total_bytes, started_at, &mut on_progress)?;
            }
            let elapsed = started_at.elapsed().as_secs_f64().max(0.001);
            on_progress(bytes_done, total_bytes, bytes_done as f64 / elapsed, source);
        }

        Ok(())
    }

    fn paste_conflict_names(sources: &[PathBuf], destination: &Path) -> Vec<String> {
        sources
            .iter()
            .filter_map(|source| {
                let file_name = source.file_name()?;
                let target = destination.join(file_name);
                if target.exists() && !same_file_path(source, &target) {
                    Some(file_name.to_string_lossy().to_string())
                } else {
                    None
                }
            })
            .collect()
    }

    fn remove_existing_target(target: &Path) -> io::Result<()> {
        if !target.exists() {
            return Ok(());
        }
        if target.is_dir() {
            fs::remove_dir_all(target)
        } else {
            fs::remove_file(target)
        }
    }

    fn same_file_path(left: &Path, right: &Path) -> bool {
        match (left.canonicalize(), right.canonicalize()) {
            (Ok(left), Ok(right)) => left == right,
            _ => left == right,
        }
    }

    fn total_size_bytes(paths: &[PathBuf]) -> u64 {
        paths.iter().map(|path| path_size_bytes(path).unwrap_or(0)).sum()
    }

    fn path_size_bytes(path: &Path) -> io::Result<u64> {
        let metadata = fs::symlink_metadata(path)?;
        if metadata.is_dir() {
            let mut total = 0_u64;
            for entry_result in fs::read_dir(path)? {
                let entry = entry_result?;
                total = total.saturating_add(path_size_bytes(&entry.path()).unwrap_or(0));
            }
            Ok(total)
        } else {
            Ok(metadata.len())
        }
    }

    fn copy_item_with_progress(
        source: &Path,
        target: &Path,
        bytes_done: &mut u64,
        bytes_total: u64,
        started_at: Instant,
        on_progress: &mut impl FnMut(u64, u64, f64, &Path),
    ) -> io::Result<()> {
        let metadata = fs::symlink_metadata(source)?;
        if metadata.is_dir() {
            fs::create_dir(target)?;
            for entry_result in fs::read_dir(source)? {
                let entry = entry_result?;
                let child_source = entry.path();
                let child_target = target.join(entry.file_name());
                copy_item_with_progress(
                    &child_source,
                    &child_target,
                    bytes_done,
                    bytes_total,
                    started_at,
                    on_progress,
                )?;
            }
            Ok(())
        } else {
            copy_file_with_progress(
                source,
                target,
                bytes_done,
                bytes_total,
                started_at,
                on_progress,
            )
        }
    }

    fn copy_file_with_progress(
        source: &Path,
        target: &Path,
        bytes_done: &mut u64,
        bytes_total: u64,
        started_at: Instant,
        on_progress: &mut impl FnMut(u64, u64, f64, &Path),
    ) -> io::Result<()> {
        let mut input = fs::File::open(source)?;
        let mut output = fs::File::create(target)?;
        let mut buffer = [0_u8; 1024 * 1024];
        loop {
            let read = input.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            output.write_all(&buffer[..read])?;
            *bytes_done = bytes_done.saturating_add(read as u64);
            let elapsed = started_at.elapsed().as_secs_f64().max(0.001);
            on_progress(*bytes_done, bytes_total, *bytes_done as f64 / elapsed, source);
        }
        Ok(())
    }

    fn move_item_with_progress(
        source: &Path,
        target: &Path,
        bytes_done: &mut u64,
        bytes_total: u64,
        started_at: Instant,
        on_progress: &mut impl FnMut(u64, u64, f64, &Path),
    ) -> io::Result<()> {
        match fs::rename(source, target) {
            Ok(()) => {
                *bytes_done = bytes_done.saturating_add(path_size_bytes(target).unwrap_or(0));
                let elapsed = started_at.elapsed().as_secs_f64().max(0.001);
                on_progress(*bytes_done, bytes_total, *bytes_done as f64 / elapsed, source);
                Ok(())
            }
            Err(rename_error) => {
                copy_item_with_progress(
                    source,
                    target,
                    bytes_done,
                    bytes_total,
                    started_at,
                    on_progress,
                )?;
                remove_item(source).map_err(|remove_error| {
                    io::Error::other(format!(
                        "moved by copy, but failed to remove original after rename failed ({rename_error}): {remove_error}"
                    ))
                })
            }
        }
    }

    fn decode_paths_json(paths_json: &str, empty_message: &str) -> io::Result<Vec<PathBuf>> {
        let values: Vec<String> = serde_json::from_str(paths_json)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))?;
        let paths: Vec<PathBuf> = values
            .into_iter()
            .map(|value| normalize_path_input(&value))
            .filter(|path| !path.as_os_str().is_empty())
            .collect();

        if paths.is_empty() {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, empty_message));
        }

        Ok(paths)
    }

    fn desktop_file_clipboard_has_files() -> bool {
        match clipboard_mime_types() {
            Ok(mime_types) => {
                mime_types.iter().any(|mime_type| {
                    mime_type == GNOME_COPIED_FILES_MIME || mime_type == URI_LIST_MIME
                })
            }
            Err(_) => false,
        }
    }

    fn write_desktop_file_clipboard(paths: &[PathBuf], cut: bool) -> io::Result<Child> {
        if paths.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "clipboard did not contain any local file paths",
            ));
        }
        if !command_exists("wl-copy") {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "wl-copy is not installed",
            ));
        }

        let action = if cut { "cut" } else { "copy" };
        let mut data = String::from(action);
        data.push('\n');
        for path in paths {
            data.push_str(&path_to_file_uri(path));
            data.push('\n');
        }

        write_clipboard_mime(GNOME_COPIED_FILES_MIME, &data)
    }

    fn clear_desktop_file_clipboard() -> io::Result<()> {
        if !command_exists("wl-copy") {
            return Ok(());
        }
        let status = Command::new("wl-copy").arg("--clear").status()?;
        if status.success() {
            Ok(())
        } else {
            Err(io::Error::other(format!("wl-copy exited with status {status}")))
        }
    }

    fn write_clipboard_mime(mime_type: &str, data: &str) -> io::Result<Child> {
        let mut child = Command::new("wl-copy")
            .arg("--foreground")
            .arg("--type")
            .arg(mime_type)
            .stdin(Stdio::piped())
            .spawn()?;
        if let Some(stdin) = child.stdin.as_mut() {
            stdin.write_all(data.as_bytes())?;
        }
        drop(child.stdin.take());
        Ok(child)
    }

    fn read_desktop_file_clipboard() -> io::Result<Option<(Vec<PathBuf>, bool)>> {
        if !command_exists("wl-paste") {
            return Ok(None);
        }

        let mime_types = clipboard_mime_types()?;
        if mime_types
            .iter()
            .any(|mime_type| mime_type == GNOME_COPIED_FILES_MIME)
        {
            let data = read_clipboard_mime(GNOME_COPIED_FILES_MIME)?;
            return parse_gnome_copied_files(&data).map(Some);
        }

        if mime_types.iter().any(|mime_type| mime_type == URI_LIST_MIME) {
            let data = read_clipboard_mime(URI_LIST_MIME)?;
            let paths = parse_uri_list(&data);
            if paths.is_empty() {
                return Ok(None);
            }
            return Ok(Some((paths, false)));
        }

        Ok(None)
    }

    fn clipboard_mime_types() -> io::Result<Vec<String>> {
        if !command_exists("wl-paste") {
            return Ok(Vec::new());
        }
        let output = Command::new("wl-paste").arg("--list-types").output()?;
        if !output.status.success() {
            return Ok(Vec::new());
        }
        Ok(String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(str::to_string)
            .collect())
    }

    fn read_clipboard_mime(mime_type: &str) -> io::Result<String> {
        let output = Command::new("wl-paste")
            .arg("--type")
            .arg(mime_type)
            .output()?;
        if !output.status.success() {
            return Err(io::Error::other(format!(
                "wl-paste exited with status {}",
                output.status
            )));
        }
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }

    fn parse_gnome_copied_files(data: &str) -> io::Result<(Vec<PathBuf>, bool)> {
        let mut lines = data.lines();
        let action = lines.next().unwrap_or("copy").trim();
        let cut = match action {
            "cut" => true,
            "copy" => false,
            _ => false,
        };
        let paths = parse_uri_lines(lines);
        if paths.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "desktop clipboard did not contain local file paths",
            ));
        }
        Ok((paths, cut))
    }

    fn parse_uri_list(data: &str) -> Vec<PathBuf> {
        parse_uri_lines(data.lines())
    }

    fn parse_uri_lines<'a>(lines: impl IntoIterator<Item = &'a str>) -> Vec<PathBuf> {
        lines
            .into_iter()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .filter_map(file_uri_to_path)
            .collect()
    }

    fn path_to_file_uri(path: &Path) -> String {
        let mut uri = String::from("file://");
        for byte in path.to_string_lossy().as_bytes() {
            match *byte {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'/' | b'-' | b'_' | b'.' | b'~' => {
                    uri.push(*byte as char);
                }
                byte => {
                    let _ = write!(uri, "%{byte:02X}");
                }
            }
        }
        uri
    }

    fn file_uri_to_path(uri: &str) -> Option<PathBuf> {
        let path = uri.strip_prefix("file://")?;
        Some(PathBuf::from(percent_decode(path)))
    }

    fn command_exists(command: &str) -> bool {
        env::var_os("PATH")
            .and_then(|paths| {
                env::split_paths(&paths)
                    .map(|path| path.join(command))
                    .find(|candidate| candidate.is_file())
            })
            .is_some()
    }

    fn validate_transfer_items_to_directory(sources: &[PathBuf], destination: &Path) -> io::Result<()> {
        if !destination.is_dir() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "drop destination is not a folder",
            ));
        }

        for source in sources {
            if is_read_only_location(&source.to_string_lossy()) {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "read-only items cannot be dropped",
                ));
            }

            let metadata = fs::symlink_metadata(source)?;
            let canonical_source = fs::canonicalize(source).unwrap_or_else(|_| source.clone());
            let canonical_destination =
                fs::canonicalize(destination).unwrap_or_else(|_| destination.to_path_buf());
            if metadata.is_dir() && canonical_destination.starts_with(&canonical_source) {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "a folder cannot be dropped into itself",
                ));
            }

            if source.file_name().is_none() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "item has no file name",
                ));
            }
        }

        Ok(())
    }

    fn remove_item(path: &Path) -> io::Result<()> {
        let metadata = fs::symlink_metadata(path)?;
        if metadata.is_dir() {
            fs::remove_dir_all(path)
        } else {
            fs::remove_file(path)
        }
    }

    fn unique_paste_destination(destination: &Path, file_name: &OsStr) -> PathBuf {
        let initial = destination.join(file_name);
        if !initial.exists() {
            return initial;
        }

        let file_name_lossy = file_name.to_string_lossy();
        let path = Path::new(file_name_lossy.as_ref());
        let stem = path
            .file_stem()
            .map(|value| value.to_string_lossy())
            .unwrap_or_else(|| Cow::Borrowed("item"));
        let extension = path.extension().map(|value| value.to_string_lossy());

        for suffix in 1_u32.. {
            let candidate_name = if let Some(extension) = extension.as_ref() {
                format!("{stem} copy {suffix}.{extension}")
            } else {
                format!("{stem} copy {suffix}")
            };
            let candidate = destination.join(candidate_name);
            if !candidate.exists() {
                return candidate;
            }
        }

        unreachable!("infinite suffix loop should always find a destination");
    }

    fn validate_new_name(name: &str) -> io::Result<&str> {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "name cannot be empty",
            ));
        }
        if trimmed == "." || trimmed == ".." {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "name is not valid",
            ));
        }
        if trimmed.contains('/') {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "name cannot contain '/'",
            ));
        }
        Ok(trimmed)
    }

    fn trash_path(path: &Path) -> io::Result<()> {
        if let Ok(()) = trash_with_gio(path) {
            return Ok(());
        }

        let file_name = path.file_name().ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "path has no final component")
        })?;

        let home = env::var_os("HOME")
            .map(PathBuf::from)
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "HOME is not set"))?;
        let trash_root = home.join(".local/share/Trash");
        let files_dir = trash_root.join("files");
        let info_dir = trash_root.join("info");
        fs::create_dir_all(&files_dir)?;
        fs::create_dir_all(&info_dir)?;

        let (trash_name, destination) = unique_trash_destination(&files_dir, file_name);
        fs::rename(path, &destination)?;
        write_trash_info(
            &info_dir.join(format!("{trash_name}.trashinfo")),
            path,
            &trash_name,
        )?;
        Ok(())
    }

    fn trash_with_gio(path: &Path) -> io::Result<()> {
        let status = Command::new("gio").arg("trash").arg(path).status()?;
        if status.success() {
            Ok(())
        } else {
            Err(io::Error::other(format!(
                "gio trash exited with status {status}"
            )))
        }
    }

    fn unique_trash_destination(files_dir: &Path, file_name: &OsStr) -> (String, PathBuf) {
        let file_name_lossy = file_name.to_string_lossy();
        let path = Path::new(file_name_lossy.as_ref());
        let stem = path
            .file_stem()
            .map(|value| value.to_string_lossy())
            .unwrap_or_else(|| Cow::Borrowed("item"));
        let extension = path.extension().map(|value| value.to_string_lossy());

        for suffix in 0_u32.. {
            let candidate_name = if suffix == 0 {
                file_name_lossy.to_string()
            } else if let Some(extension) = extension.as_ref() {
                format!("{stem} {suffix}.{extension}")
            } else {
                format!("{stem} {suffix}")
            };

            let candidate_path = files_dir.join(&candidate_name);
            if !candidate_path.exists() {
                return (candidate_name, candidate_path);
            }
        }

        unreachable!("infinite suffix loop should always find a destination");
    }

    fn write_trash_info(
        info_path: &Path,
        original_path: &Path,
        trash_name: &str,
    ) -> io::Result<()> {
        let deletion_date = deletion_timestamp();
        let encoded_path = encode_trash_path(original_path);
        let contents = format!("[Trash Info]\nPath={encoded_path}\nDeletionDate={deletion_date}\n");

        let final_info_path = unique_info_path(info_path, trash_name);
        fs::write(final_info_path, contents)
    }

    fn unique_info_path(initial_path: &Path, trash_name: &str) -> PathBuf {
        if !initial_path.exists() {
            return initial_path.to_path_buf();
        }

        let base = trash_name.strip_suffix(".trashinfo").unwrap_or(trash_name);
        for suffix in 1_u32.. {
            let candidate = initial_path.with_file_name(format!("{base} {suffix}.trashinfo"));
            if !candidate.exists() {
                return candidate;
            }
        }

        unreachable!("infinite suffix loop should always find a destination");
    }

    fn encode_trash_path(path: &Path) -> String {
        let text = path.to_string_lossy();
        let mut encoded = String::with_capacity(text.len());
        for byte in text.bytes() {
            match byte {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'/' | b'-' | b'_' | b'.' | b'~' => {
                    encoded.push(byte as char);
                }
                _ => {
                    let _ = write!(&mut encoded, "%{byte:02X}");
                }
            }
        }
        encoded
    }

    fn deletion_timestamp() -> String {
        let seconds = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|value| value.as_secs())
            .unwrap_or(0);

        Command::new("date")
            .arg("-u")
            .arg(format!("@{seconds}"))
            .arg("+%Y-%m-%dT%H:%M:%S")
            .output()
            .ok()
            .and_then(|output| String::from_utf8(output.stdout).ok())
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "1970-01-01T00:00:00".to_string())
    }

    fn normalize_path_input(input: &str) -> PathBuf {
        if is_virtual_location(input) {
            return PathBuf::from(input);
        }

        if let Some(path) = input.strip_prefix("file://") {
            return PathBuf::from(percent_decode(path));
        }

        PathBuf::from(input)
    }

    fn percent_decode(input: &str) -> String {
        let bytes = input.as_bytes();
        let mut output = String::with_capacity(input.len());
        let mut index = 0_usize;

        while index < bytes.len() {
            if bytes[index] == b'%' && index + 2 < bytes.len() {
                let hex = &input[index + 1..index + 3];
                if let Ok(value) = u8::from_str_radix(hex, 16) {
                    output.push(value as char);
                    index += 3;
                    continue;
                }
            }

            if bytes[index] == b'+' {
                output.push(' ');
            } else {
                let _ = output.write_char(bytes[index] as char);
            }
            index += 1;
        }

        output
    }

    fn initial_path() -> PathBuf {
        env::args_os()
            .nth(1)
            .map(PathBuf::from)
            .unwrap_or_else(|| env::current_dir().expect("failed to determine current directory"))
    }

    pub fn run() {
        let initial_path = initial_path();
        let mut engine = QmlEngine::new();
        let model = QObjectBox::new(FilesModel::default());
        let devices_model = QObjectBox::new(DevicesModel::default());

        {
            let pinned_model = model.pinned();
            let mut pinned = pinned_model.borrow_mut();
            pinned.current_path = QString::from(initial_path.display().to_string());
            pinned.folder_view_mode = QString::from(DEFAULT_VIEW_MODE);
            pinned.sort_field_name = QString::from(DEFAULT_SORT_FIELD);
            pinned.sort_descending = false;
            pinned.grouping_name = QString::from(DEFAULT_GROUPING);
            pinned.show_hidden = false;
            pinned.can_go_back = false;
            pinned.can_go_forward = false;
            pinned.load_path_impl(initial_path);
        }

        {
            let pinned_devices = devices_model.pinned();
            let mut pinned = pinned_devices.borrow_mut();
            pinned.reload();
        }

        engine.set_object_property(QString::from("filesModel"), model.pinned());
        engine.set_object_property(QString::from("devicesModel"), devices_model.pinned());
        engine.load_file(QString::from(qml_path().display().to_string()));
        engine.exec();
    }
}

#[cfg(not(feature = "qml"))]
mod cli_app {
    use file_ops::{EntryKind, ListOptions, list_directory};
    use std::env;
    use std::path::PathBuf;
    use std::process::ExitCode;

    pub fn run() -> ExitCode {
        let target = env::args_os()
            .nth(1)
            .map(PathBuf::from)
            .unwrap_or_else(|| env::current_dir().expect("failed to determine current directory"));

        match list_directory(&target, ListOptions::default()) {
            Ok(entries) => {
                println!("Directory: {}", target.display());
                println!(
                    "Qt/QML UI is disabled in this build. Rebuild with `--features qml` after installing Qt Quick development headers."
                );
                for entry in entries {
                    let kind = match entry.kind {
                        EntryKind::Directory => "dir ",
                        EntryKind::File => "file",
                        EntryKind::Symlink => "link",
                        EntryKind::Other => "other",
                    };

                    println!("{kind}  {:>12}  {}", entry.size_bytes, entry.name);
                }

                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("failed to list {}: {error}", target.display());
                ExitCode::FAILURE
            }
        }
    }
}

#[cfg(feature = "qml")]
fn main() {
    qml_app::run();
}

#[cfg(not(feature = "qml"))]
fn main() -> std::process::ExitCode {
    cli_app::run()
}
