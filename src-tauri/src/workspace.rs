//! Opening the Knowqore workspace: where it is by default, which folder in it
//! is the Vault, and the store that reads it.
//!
//! This is the only module that names `knowqore-store-scheda`: it opens the
//! Vault through the library of stores (`Stores.Open`), and everything after
//! it holds the store through the contract. Studio is read-only: nothing here
//! claims the writer, encodes, applies, or reindexes, and opening never makes
//! a Vault where there is none.
//!
//! A workspace is the Knowqore home, a folder that holds `data/`, the Vault;
//! the Vault is the folder with a `.git`. The person may also point at the
//! `data/` folder itself.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};

use knowqore_store_scheda::{SchedaSettings, stores};
use knowqore_stores::{Error, StoreErrorCode, StoreOpening, StoreRef, StoresAsync as _};
use serde::Serialize;
use tauri::{AppHandle, Manager, State};

/// The folder inside the Knowqore home that holds the Vault. Knowqore writes
/// it `Data` on Windows, whose file names ignore case.
const DATA_DIR: &str = "data";

/// What opening a workspace came to, as the open screen shows it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(
    tag = "state",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum WorkspaceStatus {
    /// The store opened the Vault; Studio reads it from now on.
    Open {
        /// The Knowqore home, the folder that holds `data/`.
        home: String,
        /// The Vault folder the store opened.
        vault: String,
    },
    /// The system names no home folder, so there is no default location.
    NoDefault,
    /// The folder is missing or cannot be read.
    Unreadable {
        /// The folder that was tried.
        folder: String,
        /// The system's reason.
        reason: String,
    },
    /// Neither `data/.git` nor `.git` is in the folder.
    NoVault {
        /// The folder that was tried.
        folder: String,
    },
    /// The store refused the Vault.
    Refused {
        /// The folder that was tried.
        folder: String,
        /// The store's error code, such as `NOT_A_STORE`.
        code: String,
        /// The store's own words.
        message: String,
    },
}

/// The store of the open workspace, shared by the commands that read it.
#[derive(Default)]
pub struct OpenStore(Mutex<Option<StoreRef>>);

impl OpenStore {
    /// The store of the open workspace; `None` before one has opened.
    pub fn current(&self) -> Option<StoreRef> {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

/// Tauri command `open_workspace`: opens `folder`, or the default location
/// when it is `None`, and keeps its store for the screens. A folder that
/// cannot be opened leaves the store that was open before.
#[tauri::command]
pub async fn open_workspace(
    app: AppHandle,
    open_store: State<'_, OpenStore>,
    folder: Option<String>,
) -> Result<WorkspaceStatus, String> {
    let folder = match folder {
        Some(folder) => PathBuf::from(folder),
        None => match default_location(&app) {
            Some(folder) => folder,
            None => return Ok(WorkspaceStatus::NoDefault),
        },
    };
    let (status, store) = open(&folder).await;
    if let Some(store) = store {
        *open_store.0.lock().unwrap_or_else(PoisonError::into_inner) = Some(store);
    }
    Ok(status)
}

/// The default location of the workspace on this system, as Knowqore resolves
/// it: `~/.knowqore` on Linux and macOS, `%LOCALAPPDATA%\Knowqore` on Windows.
fn default_location(app: &AppHandle) -> Option<PathBuf> {
    let path = app.path();
    default_location_for(
        cfg!(windows),
        path.home_dir().ok(),
        path.local_data_dir().ok(),
    )
}

/// [`default_location`] for a given system and directories, so all three
/// systems are tested.
fn default_location_for(
    windows: bool,
    home: Option<PathBuf>,
    local_data: Option<PathBuf>,
) -> Option<PathBuf> {
    if windows {
        local_data.map(|dir| dir.join("Knowqore"))
    } else {
        home.map(|dir| dir.join(".knowqore"))
    }
}

/// Opens the workspace at `folder`: the folder checks first, then the store.
async fn open(folder: &Path) -> (WorkspaceStatus, Option<StoreRef>) {
    let shown = folder.display().to_string();
    if let Err(error) = fs::read_dir(folder) {
        let status = WorkspaceStatus::Unreadable {
            folder: shown,
            reason: error.to_string(),
        };
        return (status, None);
    }
    let Some((home, vault)) = find_vault(folder) else {
        return (WorkspaceStatus::NoVault { folder: shown }, None);
    };
    let opening = StoreOpening::default().with_path(vault.display().to_string());
    match stores(SchedaSettings::default()).open(opening).await {
        Ok(store) => {
            let status = WorkspaceStatus::Open {
                home: home.display().to_string(),
                vault: vault.display().to_string(),
            };
            (status, Some(store))
        }
        Err(error) => (refused(shown, &error), None),
    }
}

/// The home and the Vault of a chosen folder: `data/` when it has a `.git`,
/// else the folder itself when it has one. A folder named `data` (in any
/// case) that is a Vault has its parent as the home.
fn find_vault(chosen: &Path) -> Option<(PathBuf, PathBuf)> {
    let data = chosen.join(DATA_DIR);
    if data.join(".git").exists() {
        return Some((chosen.to_owned(), data));
    }
    if !chosen.join(".git").exists() {
        return None;
    }
    let is_data = chosen
        .file_name()
        .is_some_and(|name| name.eq_ignore_ascii_case(DATA_DIR));
    let home = match chosen.parent() {
        Some(parent) if is_data => parent.to_owned(),
        _ => chosen.to_owned(),
    };
    Some((home, chosen.to_owned()))
}

/// The store's refusal, with its code and its own words (without the code
/// that `Display` writes in front of them).
fn refused(folder: String, error: &Error) -> WorkspaceStatus {
    let code = error.code().to_string();
    let message = error.to_string().replacen(&format!("{code}: "), "", 1);
    let code = if error.is(StoreErrorCode::NotAStore) {
        "NOT_A_STORE".to_owned()
    } else if error.is(StoreErrorCode::GenerationOne) {
        "GENERATION_ONE".to_owned()
    } else {
        code
    };
    WorkspaceStatus::Refused {
        folder,
        code,
        message,
    }
}

/// Registers the shared slot for the open store.
pub fn manage(app: &AppHandle) {
    app.manage(OpenStore::default());
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use super::{default_location_for, find_vault};

    #[test]
    fn the_default_location_is_dot_knowqore_in_the_home_on_linux_and_macos() {
        let home = PathBuf::from("/home/someone");
        assert_eq!(
            default_location_for(false, Some(home), Some(PathBuf::from("/unused"))),
            Some(PathBuf::from("/home/someone/.knowqore"))
        );
    }

    #[test]
    fn the_default_location_is_knowqore_in_local_app_data_on_windows() {
        let local = PathBuf::from(r"C:\Users\someone\AppData\Local");
        assert_eq!(
            default_location_for(true, Some(PathBuf::from("/unused")), Some(local.clone())),
            Some(local.join("Knowqore"))
        );
    }

    #[test]
    fn there_is_no_default_location_without_the_system_folder() {
        assert_eq!(default_location_for(false, None, None), None);
        assert_eq!(default_location_for(true, None, None), None);
    }

    fn temp_folder(name: &str) -> PathBuf {
        let folder = std::env::temp_dir()
            .join(format!("knowqore-studio-test-{}", std::process::id()))
            .join(name);
        let _ = fs::remove_dir_all(&folder);
        fs::create_dir_all(&folder).unwrap();
        folder
    }

    #[test]
    fn the_vault_of_a_home_is_its_data_folder() {
        let home = temp_folder("home");
        fs::create_dir_all(home.join("data/.git")).unwrap();
        assert_eq!(find_vault(&home), Some((home.clone(), home.join("data"))));
    }

    #[test]
    fn a_chosen_data_folder_has_its_parent_as_the_home() {
        let home = temp_folder("parent");
        let data = home.join("Data");
        fs::create_dir_all(data.join(".git")).unwrap();
        assert_eq!(find_vault(&data), Some((home, data)));
    }

    #[test]
    fn a_folder_without_a_git_holds_no_vault() {
        let folder = temp_folder("empty");
        assert_eq!(find_vault(&folder), None);
    }
}
