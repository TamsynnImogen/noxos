use std::cmp::Ordering;
use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryKind {
    File,
    Directory,
    Symlink,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileEntry {
    pub name: String,
    pub path: PathBuf,
    pub kind: EntryKind,
    pub size_bytes: u64,
    pub modified: Option<SystemTime>,
    pub hidden: bool,
}

impl FileEntry {
    #[must_use]
    pub fn extension(&self) -> Option<&str> {
        Path::new(&self.name).extension().and_then(OsStr::to_str)
    }
}

#[must_use]
pub fn group_label(entry: &FileEntry, mode: GroupMode) -> String {
    match mode {
        GroupMode::None => String::new(),
        GroupMode::Type => match entry.kind {
            EntryKind::Directory => "Folders".to_string(),
            EntryKind::Symlink => "Symlinks".to_string(),
            EntryKind::Other => "Other".to_string(),
            EntryKind::File => match entry.extension() {
                Some(ext) if !ext.is_empty() => format!("{} files", ext.to_ascii_uppercase()),
                _ => "Files without extension".to_string(),
            },
        },
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortField {
    Name,
    Size,
    Kind,
    Modified,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupMode {
    None,
    Type,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ListOptions {
    pub include_hidden: bool,
    pub sort_field: SortField,
    pub directories_first: bool,
    pub descending: bool,
    pub group_mode: GroupMode,
}

impl Default for ListOptions {
    fn default() -> Self {
        Self {
            include_hidden: false,
            sort_field: SortField::Name,
            directories_first: true,
            descending: false,
            group_mode: GroupMode::None,
        }
    }
}

pub fn list_directory(path: impl AsRef<Path>, options: ListOptions) -> io::Result<Vec<FileEntry>> {
    let path = path.as_ref();
    let mut entries = Vec::new();

    for entry_result in fs::read_dir(path)? {
        let entry = entry_result?;
        let file_name = entry.file_name();
        let name = file_name.to_string_lossy().into_owned();
        let hidden = name.starts_with('.');

        if hidden && !options.include_hidden {
            continue;
        }

        let metadata = fs::symlink_metadata(entry.path())?;
        let file_type = metadata.file_type();
        let kind = if file_type.is_dir() {
            EntryKind::Directory
        } else if file_type.is_file() {
            EntryKind::File
        } else if file_type.is_symlink() {
            EntryKind::Symlink
        } else {
            EntryKind::Other
        };

        entries.push(FileEntry {
            name,
            path: entry.path(),
            kind,
            size_bytes: metadata.len(),
            modified: metadata.modified().ok(),
            hidden,
        });
    }

    entries.sort_by(|left, right| compare_entries(left, right, options));
    Ok(entries)
}

fn compare_entries(left: &FileEntry, right: &FileEntry, options: ListOptions) -> Ordering {
    if options.group_mode == GroupMode::Type {
        let grouped =
            group_label(left, options.group_mode).cmp(&group_label(right, options.group_mode));
        if grouped != Ordering::Equal {
            return grouped;
        }
    }

    if options.directories_first {
        match (
            left.kind == EntryKind::Directory,
            right.kind == EntryKind::Directory,
        ) {
            (true, false) => return Ordering::Less,
            (false, true) => return Ordering::Greater,
            _ => {}
        }
    }

    let primary = match options.sort_field {
        SortField::Name => left.name.to_lowercase().cmp(&right.name.to_lowercase()),
        SortField::Size => left
            .size_bytes
            .cmp(&right.size_bytes)
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase())),
        SortField::Kind => left
            .kind
            .cmp(&right.kind)
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase())),
        SortField::Modified => left
            .modified
            .cmp(&right.modified)
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase())),
    };

    let primary = if options.descending {
        primary.reverse()
    } else {
        primary
    };

    if primary == Ordering::Equal {
        left.path.cmp(&right.path)
    } else {
        primary
    }
}

impl Ord for EntryKind {
    fn cmp(&self, other: &Self) -> Ordering {
        use EntryKind::{Directory, File, Other, Symlink};

        let rank = |kind: &EntryKind| match kind {
            Directory => 0_u8,
            File => 1_u8,
            Symlink => 2_u8,
            Other => 3_u8,
        };

        rank(self).cmp(&rank(other))
    }
}

impl PartialOrd for EntryKind {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[cfg(test)]
mod tests {
    use super::{
        EntryKind, FileEntry, GroupMode, ListOptions, SortField, compare_entries, group_label,
    };
    use std::cmp::Ordering;
    use std::path::PathBuf;
    use std::time::{Duration, SystemTime};

    fn entry(name: &str, kind: EntryKind, size_bytes: u64, modified_secs: u64) -> FileEntry {
        FileEntry {
            name: name.to_string(),
            path: PathBuf::from(name),
            kind,
            size_bytes,
            modified: Some(SystemTime::UNIX_EPOCH + Duration::from_secs(modified_secs)),
            hidden: name.starts_with('.'),
        }
    }

    #[test]
    fn directories_are_sorted_first_when_enabled() {
        let left = entry("folder", EntryKind::Directory, 0, 1);
        let right = entry("file.txt", EntryKind::File, 0, 1);

        let ordering = compare_entries(&left, &right, ListOptions::default());
        assert_eq!(ordering, Ordering::Less);
    }

    #[test]
    fn size_sort_falls_back_to_name() {
        let left = entry("alpha.txt", EntryKind::File, 10, 5);
        let right = entry("beta.txt", EntryKind::File, 10, 4);

        let ordering = compare_entries(
            &left,
            &right,
            ListOptions {
                include_hidden: false,
                sort_field: SortField::Size,
                directories_first: false,
                descending: false,
                group_mode: GroupMode::None,
            },
        );

        assert_eq!(ordering, Ordering::Less);
    }

    #[test]
    fn modified_sort_uses_timestamp() {
        let older = entry("older.txt", EntryKind::File, 1, 1);
        let newer = entry("newer.txt", EntryKind::File, 1, 9);

        let ordering = compare_entries(
            &older,
            &newer,
            ListOptions {
                include_hidden: false,
                sort_field: SortField::Modified,
                directories_first: false,
                descending: false,
                group_mode: GroupMode::None,
            },
        );

        assert_eq!(ordering, Ordering::Less);
    }

    #[test]
    fn descending_sort_reverses_primary_order() {
        let left = entry("alpha.txt", EntryKind::File, 1, 1);
        let right = entry("beta.txt", EntryKind::File, 1, 1);

        let ordering = compare_entries(
            &left,
            &right,
            ListOptions {
                include_hidden: false,
                sort_field: SortField::Name,
                directories_first: false,
                descending: true,
                group_mode: GroupMode::None,
            },
        );

        assert_eq!(ordering, Ordering::Greater);
    }

    #[test]
    fn type_grouping_keeps_kinds_together() {
        let file = entry("alpha.txt", EntryKind::File, 1, 1);
        let folder = entry("folder", EntryKind::Directory, 1, 1);

        let ordering = compare_entries(
            &file,
            &folder,
            ListOptions {
                include_hidden: false,
                sort_field: SortField::Name,
                directories_first: false,
                descending: false,
                group_mode: GroupMode::Type,
            },
        );

        assert_eq!(ordering, Ordering::Greater);
    }

    #[test]
    fn type_grouping_uses_extension_label() {
        let png = entry("image.png", EntryKind::File, 1, 1);
        let zip = entry("archive.zip", EntryKind::File, 1, 1);

        assert_eq!(group_label(&png, GroupMode::Type), "PNG files");
        assert_eq!(group_label(&zip, GroupMode::Type), "ZIP files");
    }
}
