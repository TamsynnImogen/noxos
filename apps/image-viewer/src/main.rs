#[cfg(feature = "qml")]
mod qml_app {
    use file_ops::{EntryKind, GroupMode, ListOptions, SortField, list_directory};
    use qmetaobject::prelude::*;
    use serde::{Deserialize, Serialize};
    use std::collections::{HashMap, HashSet, hash_map::DefaultHasher};
    use std::env;
    use std::ffi::OsStr;
    use std::fmt::Write as _;
    use std::fs;
    use std::hash::{Hash, Hasher};
    use std::io;
    use std::path::{Path, PathBuf};
    use std::process::{Command, Stdio};

    const ARCHIVE_URI_PREFIX: &str = "archive://";
    const DEFAULT_SLIDESHOW_INTERVAL_SECONDS: i32 = 4;

    #[derive(Clone, Default)]
    struct ViewerItem {
        name: String,
        virtual_path: String,
        source_path: PathBuf,
        source_url: String,
        media_kind: String,
        format_name: String,
        width: i32,
        height: i32,
        from_archive: bool,
    }

    impl ViewerItem {
        fn dimensions_text(&self) -> String {
            if self.width > 0 && self.height > 0 {
                format!("{} x {}", self.width, self.height)
            } else {
                "-".to_string()
            }
        }
    }

    #[derive(Serialize)]
    struct ViewerItemJson {
        name: String,
        path: String,
        source_url: String,
        media_kind: String,
        from_archive: bool,
    }

    #[derive(Deserialize)]
    struct TranslationResult {
        detected_language: String,
        extracted_text: String,
        translated_text: String,
        translation_available: bool,
        translation_error: String,
    }

    #[derive(QObject, Default)]
    struct ImageViewerModel {
        #[qt_base_class = "QObject"]
        base: qmetaobject::QObjectCppWrapper,
        entries: Vec<ViewerItem>,
        current_name: qt_property!(QString; NOTIFY current_changed),
        current_source_url: qt_property!(QString; NOTIFY current_changed),
        current_media_kind: qt_property!(QString; NOTIFY current_changed),
        current_format: qt_property!(QString; NOTIFY current_changed),
        current_dimensions: qt_property!(QString; NOTIFY current_changed),
        current_index: qt_property!(i32; NOTIFY current_changed),
        current_count: qt_property!(i32; NOTIFY current_changed),
        current_rotation: qt_property!(i32; NOTIFY current_changed),
        current_path: qt_property!(QString; NOTIFY current_changed),
        window_title: qt_property!(QString; NOTIFY current_changed),
        has_image: qt_property!(bool; NOTIFY current_changed),
        can_go_previous: qt_property!(bool; NOTIFY current_changed),
        can_go_next: qt_property!(bool; NOTIFY current_changed),
        status_text: qt_property!(QString; NOTIFY status_text_changed),
        status_text_changed: qt_signal!(),
        error_message: qt_property!(QString; NOTIFY error_message_changed),
        error_message_changed: qt_signal!(),
        translation_target_language: qt_property!(QString; NOTIFY translation_changed),
        translation_detected_language: qt_property!(QString; NOTIFY translation_changed),
        translation_extracted_text: qt_property!(QString; NOTIFY translation_changed),
        translation_text: qt_property!(QString; NOTIFY translation_changed),
        translation_status: qt_property!(QString; NOTIFY translation_changed),
        translation_busy: qt_property!(bool; NOTIFY translation_changed),
        translation_changed: qt_signal!(),
        slideshow_running: qt_property!(bool; NOTIFY slideshow_running_changed),
        slideshow_running_changed: qt_signal!(),
        slideshow_interval_seconds: qt_property!(i32; NOTIFY slideshow_interval_seconds_changed),
        slideshow_interval_seconds_changed: qt_signal!(),
        items_json: qt_property!(QString; NOTIFY items_json_changed),
        items_json_changed: qt_signal!(),
        current_changed: qt_signal!(),
        open_target: qt_method!(
            fn open_target(&mut self, path: QString) {
                self.open_target_impl(normalize_path_input(&path.to_string()));
            }
        ),
        previous_image: qt_method!(
            fn previous_image(&mut self) {
                if self.can_go_previous {
                    self.select_index_internal((self.current_index - 1) as usize);
                }
            }
        ),
        next_image: qt_method!(
            fn next_image(&mut self) {
                if self.can_go_next {
                    self.select_index_internal((self.current_index + 1) as usize);
                }
            }
        ),
        advance_slideshow: qt_method!(
            fn advance_slideshow(&mut self) {
                if self.entries.is_empty() {
                    return;
                }

                let next_index = if self.current_index + 1 >= self.entries.len() as i32 {
                    0
                } else {
                    self.current_index + 1
                };
                self.select_index_internal(next_index as usize);
            }
        ),
        select_index: qt_method!(
            fn select_index(&mut self, index: i32) {
                if index < 0 {
                    return;
                }
                self.select_index_internal(index as usize);
            }
        ),
        rotate_left: qt_method!(
            fn rotate_left(&mut self) {
                if !self.has_image {
                    return;
                }
                self.current_rotation = normalize_rotation(self.current_rotation - 90);
                self.current_changed();
            }
        ),
        rotate_right: qt_method!(
            fn rotate_right(&mut self) {
                if !self.has_image {
                    return;
                }
                self.current_rotation = normalize_rotation(self.current_rotation + 90);
                self.current_changed();
            }
        ),
        reset_rotation: qt_method!(
            fn reset_rotation(&mut self) {
                if !self.has_image || self.current_rotation == 0 {
                    return;
                }
                self.current_rotation = 0;
                self.current_changed();
            }
        ),
        set_slideshow_running: qt_method!(
            fn set_slideshow_running(&mut self, running: bool) {
                if self.slideshow_running == running {
                    return;
                }
                self.slideshow_running = running;
                self.slideshow_running_changed();
                self.set_status(if running {
                    "Slideshow started".to_string()
                } else {
                    "Slideshow paused".to_string()
                });
            }
        ),
        set_slideshow_interval_seconds: qt_method!(
            fn set_slideshow_interval_seconds(&mut self, seconds: i32) {
                let clamped = seconds.clamp(2, 60);
                if self.slideshow_interval_seconds == clamped {
                    return;
                }
                self.slideshow_interval_seconds = clamped;
                self.slideshow_interval_seconds_changed();
                self.set_status(format!("Slideshow interval set to {clamped}s"));
            }
        ),
        set_translation_target_language: qt_method!(
            fn set_translation_target_language(&mut self, language: QString) {
                let trimmed = language.to_string().trim().to_string();
                let next_value = if trimmed.is_empty() {
                    "en".to_string()
                } else {
                    trimmed
                };
                if self.translation_target_language.to_string() == next_value {
                    return;
                }
                self.translation_target_language = QString::from(next_value);
                self.translation_changed();
            }
        ),
        translate_current_text: qt_method!(
            fn translate_current_text(&mut self) {
                self.translate_current_text_impl();
            }
        ),
        export_current: qt_method!(
            fn export_current(&mut self, destination: QString) {
                let destination = normalize_path_input(&destination.to_string());
                match self.export_current_impl(&destination) {
                    Ok(()) => {
                        self.error_message = QString::default();
                        self.error_message_changed();
                        self.set_status(format!("Saved copy to {}", destination.display()));
                    }
                    Err(error) => {
                        self.error_message = QString::from(format!("Failed to save copy: {error}"));
                        self.error_message_changed();
                    }
                }
            }
        ),
    }

    impl ImageViewerModel {
        fn open_target_impl(&mut self, target: PathBuf) {
            match load_collection(&target) {
                Ok((entries, current_index)) => {
                    self.entries = entries;
                    self.items_json = QString::from(build_items_json(&self.entries));
                    self.items_json_changed();
                    self.current_rotation = 0;
                    self.error_message = QString::default();
                    self.error_message_changed();

                    if self.entries.is_empty() {
                        self.current_index = -1;
                        self.update_current_properties();
                        self.set_status("No supported media found in this location".to_string());
                    } else {
                        self.select_index_internal(current_index);
                        self.set_status(format!("Loaded {} item(s)", self.entries.len()));
                    }
                }
                Err(error) => {
                    self.entries.clear();
                    self.items_json = QString::from("[]");
                    self.items_json_changed();
                    self.current_index = -1;
                    self.current_rotation = 0;
                    self.update_current_properties();
                    self.error_message =
                        QString::from(format!("Failed to open image collection: {error}"));
                    self.error_message_changed();
                    self.set_status(String::new());
                }
            }
        }

        fn select_index_internal(&mut self, index: usize) {
            if self.entries.is_empty() {
                self.current_index = -1;
                self.current_rotation = 0;
                self.clear_translation_results();
                self.update_current_properties();
                return;
            }

            let clamped_index = index.min(self.entries.len().saturating_sub(1));
            self.current_index = clamped_index as i32;
            self.current_rotation = 0;
            self.clear_translation_results();

            if let Err(error) = self.ensure_current_metadata() {
                self.error_message = QString::from(format!("Failed to inspect image: {error}"));
                self.error_message_changed();
            } else {
                self.error_message = QString::default();
                self.error_message_changed();
            }

            self.update_current_properties();
        }

        fn ensure_current_metadata(&mut self) -> io::Result<()> {
            let Some(item) = self.entries.get_mut(self.current_index as usize) else {
                return Ok(());
            };

            if item.width > 0 && item.height > 0 && !item.format_name.is_empty() {
                return Ok(());
            }

            let (format_name, width, height) =
                probe_media_metadata(&item.source_path, &item.media_kind)?;
            item.format_name = format_name;
            item.width = width;
            item.height = height;
            Ok(())
        }

        fn update_current_properties(&mut self) {
            self.current_count = self.entries.len() as i32;
            self.has_image = !self.entries.is_empty() && self.current_index >= 0;
            self.can_go_previous = self.has_image && self.current_index > 0;
            self.can_go_next = self.has_image && (self.current_index + 1) < self.current_count;

            if let Some(item) = self.entries.get(self.current_index.max(0) as usize) {
                self.current_name = QString::from(item.name.clone());
                self.current_source_url = QString::from(item.source_url.clone());
                self.current_media_kind = QString::from(item.media_kind.clone());
                self.current_format = QString::from(if item.format_name.is_empty() {
                    "-".to_string()
                } else {
                    item.format_name.clone()
                });
                self.current_dimensions = QString::from(item.dimensions_text());
                self.current_path = QString::from(item.virtual_path.clone());
                self.window_title = QString::from(format!(
                    "{} ({}/{})",
                    item.name,
                    self.current_index + 1,
                    self.current_count
                ));
            } else {
                self.current_name = QString::default();
                self.current_source_url = QString::default();
                self.current_media_kind = QString::default();
                self.current_format = QString::from("-");
                self.current_dimensions = QString::from("-");
                self.current_path = QString::default();
                self.window_title = QString::from("Image Viewer");
            }

            self.current_changed();
        }

        fn export_current_impl(&self, destination: &Path) -> io::Result<()> {
            let Some(item) = self.entries.get(self.current_index.max(0) as usize) else {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "no image selected",
                ));
            };

            if item.media_kind == "video" {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "video export/conversion is not supported in this build",
                ));
            }

            if destination.as_os_str().is_empty() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "destination path is empty",
                ));
            }

            if destination.extension().and_then(OsStr::to_str).is_none() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "destination needs a file extension like .png or .jpg",
                ));
            }

            if destination == item.source_path {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "pick a new path instead of overwriting the current source",
                ));
            }

            let parent = destination.parent().ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "destination has no parent directory",
                )
            })?;

            if !parent.exists() {
                return Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("destination folder does not exist: {}", parent.display()),
                ));
            }

            export_image(&item.source_path, destination, self.current_rotation)
        }

        fn translate_current_text_impl(&mut self) {
            let Some(item) = self
                .entries
                .get(self.current_index.max(0) as usize)
                .cloned()
            else {
                self.translation_status = QString::from("No still image selected");
                self.translation_changed();
                return;
            };

            if item.media_kind != "image" {
                self.translation_status =
                    QString::from("Translation is only available for still images");
                self.translation_changed();
                return;
            }

            let target_language = self.translation_target_language.to_string();
            let target_language = if target_language.trim().is_empty() {
                "en".to_string()
            } else {
                target_language.trim().to_string()
            };

            self.translation_target_language = QString::from(target_language.clone());
            self.translation_busy = true;
            self.translation_status = QString::from("Running OCR...");
            self.translation_changed();

            match translate_still_image(&item.source_path, &target_language) {
                Ok(result) => {
                    self.translation_detected_language = QString::from(result.detected_language);
                    self.translation_extracted_text = QString::from(result.extracted_text);
                    self.translation_text = QString::from(result.translated_text);
                    self.translation_status = if result.translation_available {
                        QString::from(format!("Translated to {}", target_language))
                    } else if !result.translation_error.is_empty() {
                        QString::from(format!(
                            "OCR complete. Translation skipped: {}",
                            result.translation_error
                        ))
                    } else {
                        QString::from("OCR complete. Translation unavailable")
                    };
                    self.error_message = QString::default();
                    self.error_message_changed();
                }
                Err(error) => {
                    self.translation_detected_language = QString::default();
                    self.translation_extracted_text = QString::default();
                    self.translation_text = QString::default();
                    self.translation_status = QString::from(format!("Translation failed: {error}"));
                }
            }

            self.translation_busy = false;
            self.translation_changed();
        }

        fn clear_translation_results(&mut self) {
            self.translation_detected_language = QString::default();
            self.translation_extracted_text = QString::default();
            self.translation_text = QString::default();
            self.translation_status = QString::default();
            self.translation_busy = false;
            self.translation_changed();
        }

        fn set_status(&mut self, message: String) {
            self.status_text = QString::from(message);
            self.status_text_changed();
        }
    }

    fn qml_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("qml/Main.qml")
    }

    fn build_items_json(items: &[ViewerItem]) -> String {
        serde_json::to_string(
            &items
                .iter()
                .map(|item| ViewerItemJson {
                    name: item.name.clone(),
                    path: item.virtual_path.clone(),
                    source_url: item.source_url.clone(),
                    media_kind: item.media_kind.clone(),
                    from_archive: item.from_archive,
                })
                .collect::<Vec<_>>(),
        )
        .unwrap_or_else(|_| "[]".to_string())
    }

    fn load_collection(target: &Path) -> io::Result<(Vec<ViewerItem>, usize)> {
        if let Some((archive_path, inner_path)) = parse_archive_uri(&target.to_string_lossy()) {
            load_archive_collection(&archive_path, &inner_path)
        } else if is_archive_extension(target) {
            load_archive_collection(target, "")
        } else {
            load_directory_collection(target)
        }
    }

    fn load_directory_collection(target: &Path) -> io::Result<(Vec<ViewerItem>, usize)> {
        let target_path = resolve_display_path(target);
        let current_target = if target_path.is_dir() {
            None
        } else {
            Some(target_path.clone())
        };
        let directory = if target_path.is_dir() {
            target_path.clone()
        } else {
            target_path
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or_else(|| PathBuf::from("/"))
        };

        let entries = list_directory(
            &directory,
            ListOptions {
                include_hidden: true,
                sort_field: SortField::Name,
                directories_first: false,
                descending: false,
                group_mode: GroupMode::None,
            },
        )?;

        let items = entries
            .into_iter()
            .filter(|entry| {
                matches!(entry.kind, EntryKind::File | EntryKind::Symlink)
                    && is_supported_media(&entry.path)
            })
            .map(viewer_item_from_path)
            .collect::<Vec<_>>();

        let current_index = current_target
            .as_ref()
            .and_then(|target_path| {
                items
                    .iter()
                    .position(|item| resolve_display_path(&item.source_path) == *target_path)
            })
            .unwrap_or(0);

        Ok((items, current_index))
    }

    fn viewer_item_from_path(entry: file_ops::FileEntry) -> ViewerItem {
        ViewerItem {
            name: entry.name,
            virtual_path: entry.path.display().to_string(),
            source_url: file_url_for_path(&entry.path),
            media_kind: media_kind_for_path(&entry.path).to_string(),
            source_path: entry.path,
            format_name: String::new(),
            width: 0,
            height: 0,
            from_archive: false,
        }
    }

    fn load_archive_collection(
        archive_path: &Path,
        inner_path: &str,
    ) -> io::Result<(Vec<ViewerItem>, usize)> {
        let normalized = inner_path.trim_matches('/');
        let folder_inner = if normalized.is_empty() {
            String::new()
        } else if is_supported_media(Path::new(normalized)) {
            archive_parent_path(normalized)
        } else {
            normalized.to_string()
        };

        let items = list_archive_images(archive_path, &folder_inner)?;
        let current_index = if normalized.is_empty() || !is_supported_media(Path::new(normalized)) {
            0
        } else {
            let target_uri = archive_uri(archive_path, normalized);
            items
                .iter()
                .position(|item| item.virtual_path == target_uri)
                .unwrap_or(0)
        };

        Ok((items, current_index))
    }

    fn list_archive_images(archive_path: &Path, folder_inner: &str) -> io::Result<Vec<ViewerItem>> {
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
        let mut items = Vec::new();
        let mut seen = HashSet::new();
        let mut current: HashMap<String, String> = HashMap::new();

        for line in stdout.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                if let Some(item) = archive_image_from_block(archive_path, folder_inner, &current)?
                {
                    if seen.insert(item.virtual_path.clone()) {
                        items.push(item);
                    }
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

        if let Some(item) = archive_image_from_block(archive_path, folder_inner, &current)? {
            if seen.insert(item.virtual_path.clone()) {
                items.push(item);
            }
        }

        items.sort_by(|left, right| left.name.to_lowercase().cmp(&right.name.to_lowercase()));
        Ok(items)
    }

    fn archive_image_from_block(
        archive_path: &Path,
        folder_inner: &str,
        block: &HashMap<String, String>,
    ) -> io::Result<Option<ViewerItem>> {
        let path = match block.get("Path") {
            Some(path) => path,
            None => return Ok(None),
        };

        if block.get("Folder").map(String::as_str).unwrap_or("-") == "+" {
            return Ok(None);
        }

        let normalized_folder = folder_inner.trim_matches('/');
        let prefix = if normalized_folder.is_empty() {
            String::new()
        } else {
            format!("{normalized_folder}/")
        };

        if !prefix.is_empty() && !path.starts_with(&prefix) {
            return Ok(None);
        }

        let remaining = if prefix.is_empty() {
            path.as_str()
        } else {
            &path[prefix.len()..]
        };

        if remaining.is_empty() || remaining.contains('/') || !is_supported_media(Path::new(path)) {
            return Ok(None);
        }

        let extracted_path = archive_member_cache_path(archive_path, path);
        if !extracted_path.exists() {
            extract_archive_member_to_cache(archive_path, path, &extracted_path)?;
        }

        Ok(Some(ViewerItem {
            name: remaining.to_string(),
            virtual_path: archive_uri(archive_path, path),
            source_url: file_url_for_path(&extracted_path),
            source_path: extracted_path,
            media_kind: media_kind_for_path(Path::new(path)).to_string(),
            format_name: String::new(),
            width: 0,
            height: 0,
            from_archive: true,
        }))
    }

    fn probe_media_metadata(path: &Path, media_kind: &str) -> io::Result<(String, i32, i32)> {
        if media_kind == "video" {
            let format_name = path
                .extension()
                .and_then(OsStr::to_str)
                .map(|ext| ext.to_ascii_uppercase())
                .unwrap_or_else(|| "VIDEO".to_string());
            return Ok((format_name, 0, 0));
        }

        let output = Command::new("identify")
            .args(["-format", "%m|%w|%h", path.to_string_lossy().as_ref()])
            .output()?;

        if !output.status.success() {
            return Err(io::Error::other(
                String::from_utf8_lossy(&output.stderr).trim().to_string(),
            ));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut parts = stdout.trim().split('|');
        let format_name = parts.next().unwrap_or("-").to_string();
        let width = parts
            .next()
            .and_then(|value| value.parse::<i32>().ok())
            .unwrap_or(0);
        let height = parts
            .next()
            .and_then(|value| value.parse::<i32>().ok())
            .unwrap_or(0);
        Ok((format_name, width, height))
    }

    fn export_image(source: &Path, destination: &Path, rotation_degrees: i32) -> io::Result<()> {
        let mut command = Command::new("convert");
        command.arg(source);

        let normalized_rotation = normalize_rotation(rotation_degrees);
        if normalized_rotation != 0 {
            command.arg("-rotate").arg(normalized_rotation.to_string());
        }

        let output = command.arg(destination).output()?;
        if output.status.success() {
            Ok(())
        } else {
            Err(io::Error::other(
                String::from_utf8_lossy(&output.stderr).trim().to_string(),
            ))
        }
    }

    fn translate_still_image(path: &Path, target_language: &str) -> io::Result<TranslationResult> {
        let script_path = translation_helper_script_path();
        if !script_path.is_file() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!(
                    "translation helper script is missing: {}",
                    script_path.display()
                ),
            ));
        }

        let output = Command::new("python3")
            .arg(script_path)
            .arg(path)
            .arg(target_language)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let message = if !stderr.is_empty() {
                stderr
            } else if !stdout.is_empty() {
                stdout
            } else {
                format!("translation helper exited with status {}", output.status)
            };
            return Err(io::Error::other(message));
        }

        serde_json::from_slice::<TranslationResult>(&output.stdout)
            .map_err(|error| io::Error::other(format!("invalid translation output: {error}")))
    }

    fn translation_helper_script_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("scripts")
            .join("translate_still_image.py")
    }

    fn normalize_rotation(rotation_degrees: i32) -> i32 {
        rotation_degrees.rem_euclid(360)
    }

    fn is_image_extension(path: &Path) -> bool {
        matches!(
            path.extension().and_then(OsStr::to_str).map(|ext| ext.to_ascii_lowercase()),
            Some(ext) if matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp" | "svg")
        )
    }

    fn is_animated_extension(path: &Path) -> bool {
        matches!(
            path.extension().and_then(OsStr::to_str).map(|ext| ext.to_ascii_lowercase()),
            Some(ext) if matches!(ext.as_str(), "gif" | "webp")
        )
    }

    fn is_video_extension(path: &Path) -> bool {
        matches!(
            path.extension().and_then(OsStr::to_str).map(|ext| ext.to_ascii_lowercase()),
            Some(ext) if matches!(ext.as_str(), "mp4" | "mkv" | "webm" | "mov" | "avi" | "m4v" | "ogv")
        )
    }

    fn is_supported_media(path: &Path) -> bool {
        is_image_extension(path) || is_video_extension(path)
    }

    fn media_kind_for_path(path: &Path) -> &'static str {
        if is_video_extension(path) {
            "video"
        } else if is_animated_extension(path) {
            "animated"
        } else {
            "image"
        }
    }

    fn is_archive_extension(path: &Path) -> bool {
        matches!(
            path.extension().and_then(OsStr::to_str).map(|ext| ext.to_ascii_lowercase()),
            Some(ext) if matches!(
                ext.as_str(),
                "zip" | "rar" | "7z" | "tar" | "gz" | "bz2" | "xz" | "tgz" | "tbz2" | "txz"
            )
        )
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

    fn extract_archive_member_to_cache(
        archive_path: &Path,
        inner_path: &str,
        destination: &Path,
    ) -> io::Result<()> {
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }

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
                String::from_utf8_lossy(&output.stderr).trim().to_string(),
            ));
        }

        fs::write(destination, output.stdout)
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

    fn file_url_for_path(path: &Path) -> String {
        format!("file://{}", encode_path(path))
    }

    fn encode_path(path: &Path) -> String {
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

    fn resolve_display_path(path: &Path) -> PathBuf {
        fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
    }

    fn normalize_path_input(input: &str) -> PathBuf {
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

    fn initial_target() -> Option<PathBuf> {
        env::args_os().nth(1).map(PathBuf::from)
    }

    pub fn run() -> std::process::ExitCode {
        let initial_target = initial_target();
        let mut engine = QmlEngine::new();
        let model = QObjectBox::new(ImageViewerModel::default());

        {
            let pinned_model = model.pinned();
            let mut pinned = pinned_model.borrow_mut();
            pinned.current_index = -1;
            pinned.current_count = 0;
            pinned.current_rotation = 0;
            pinned.window_title = QString::from("Image Viewer");
            pinned.current_format = QString::from("-");
            pinned.current_dimensions = QString::from("-");
            pinned.translation_target_language = QString::from("en");
            pinned.translation_status = QString::from("Translation is available for still images");
            pinned.translation_busy = false;
            pinned.slideshow_running = false;
            pinned.slideshow_interval_seconds = DEFAULT_SLIDESHOW_INTERVAL_SECONDS;
            if let Some(initial_target) = initial_target {
                pinned.open_target_impl(initial_target);
            } else {
                pinned.status_text = QString::from("Open an image or archive to begin");
            }
        }

        engine.set_object_property(QString::from("imageViewerModel"), model.pinned());
        engine.load_file(QString::from(qml_path().display().to_string()));
        engine.exec();
        std::process::ExitCode::SUCCESS
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
