use crate::model::{
    MAX_PLAN_BYTES, MAX_SNAPSHOT_BYTES, Receipt, SCHEMA_VERSION, SnapshotEntry, SnapshotManifest,
    Tombstones, UciConfig,
};
use crate::{EngineError, FaultInjector, FaultPoint};
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::ffi::OsStr;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::os::fd::{AsRawFd, RawFd};
#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TxnPaths {
    root: PathBuf,
    config_root: PathBuf,
}

impl TxnPaths {
    pub fn new(root: impl Into<PathBuf>, config_root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            config_root: config_root.into(),
        }
    }

    pub fn production() -> Self {
        Self::new("/etc/mjolnir/txn", "/etc/config")
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn lock(&self) -> PathBuf {
        self.root.join("lock")
    }

    pub fn journal(&self) -> PathBuf {
        self.root.join("journal.json")
    }

    pub fn receipts(&self) -> PathBuf {
        self.root.join("receipts")
    }

    pub fn receipt(&self, id: &str) -> PathBuf {
        self.receipts().join(format!("{id}.json"))
    }

    pub fn tombstones(&self) -> PathBuf {
        self.root.join("tombstones.json")
    }

    pub fn active(&self) -> PathBuf {
        self.root.join("active")
    }

    pub fn transaction(&self, id: &str) -> PathBuf {
        self.active().join(id)
    }

    pub fn plan(&self, id: &str) -> PathBuf {
        self.transaction(id).join("plan.json")
    }

    pub fn snapshot(&self, id: &str) -> PathBuf {
        self.transaction(id).join("snapshot")
    }

    pub fn config(&self, config: UciConfig) -> PathBuf {
        self.config_root.join(config.file_name())
    }
}

/// Kernel-owned process lock. The lock is released when this file description
/// is closed or the process dies.
#[derive(Debug)]
pub struct NodeLock {
    handle: LockHandle,
}

#[derive(Debug)]
enum LockHandle {
    Owned(File),
    Inherited,
}

impl NodeLock {
    pub fn acquire(paths: &TxnPaths) -> Result<Self, EngineError> {
        ensure_dir(paths.root())?;
        let mut options = OpenOptions::new();
        options.read(true).write(true).create(true);
        #[cfg(unix)]
        options.mode(0o600);
        let file = options.open(paths.lock())?;
        // SAFETY: flock only observes the valid descriptor owned by `file`.
        let result = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
        if result != 0 {
            let error = io::Error::last_os_error();
            if error.kind() == io::ErrorKind::WouldBlock {
                return Err(EngineError::Busy);
            }
            return Err(error.into());
        }
        Ok(Self {
            handle: LockHandle::Owned(file),
        })
    }

    /// Adopt a lock file description inherited from a supervising launcher.
    ///
    /// This deliberately never opens the lock path. `flock` is repeated on the
    /// inherited open file description so a helper cannot accidentally run
    /// without the launcher's node-wide lock.
    pub fn inherit(raw_fd: RawFd) -> Result<Self, EngineError> {
        if raw_fd < 0 {
            return Err(EngineError::Storage("invalid inherited lock fd".into()));
        }
        // SAFETY: F_GETFD only validates the caller-provided descriptor.
        if unsafe { libc::fcntl(raw_fd, libc::F_GETFD) } < 0 {
            return Err(EngineError::Io(io::Error::last_os_error()));
        }
        // SAFETY: the descriptor was validated above. An inherited descriptor
        // refers to the same open file description locked by the parent shell.
        let result = unsafe { libc::flock(raw_fd, libc::LOCK_EX | libc::LOCK_NB) };
        if result != 0 {
            let error = io::Error::last_os_error();
            if error.kind() == io::ErrorKind::WouldBlock {
                return Err(EngineError::Busy);
            }
            return Err(error.into());
        }
        Ok(Self {
            handle: LockHandle::Inherited,
        })
    }
}

impl Drop for NodeLock {
    fn drop(&mut self) {
        if let LockHandle::Owned(file) = &self.handle {
            // SAFETY: the descriptor remains valid until `file` is dropped.
            unsafe {
                libc::flock(file.as_raw_fd(), libc::LOCK_UN);
            }
        }
    }
}

pub(crate) fn initialize(paths: &TxnPaths) -> Result<(), EngineError> {
    ensure_dir(paths.root())?;
    ensure_dir(&paths.receipts())?;
    ensure_dir(&paths.active())?;
    if !paths.tombstones().exists() {
        atomic_json(&paths.tombstones(), &Tombstones::default())?;
    }
    Ok(())
}

pub(crate) fn ensure_dir(path: &Path) -> Result<(), EngineError> {
    if path.is_dir() {
        return Ok(());
    }
    fs::create_dir_all(path)?;
    sync_dir(path)?;
    if let Some(parent) = path.parent() {
        sync_dir(parent)?;
    }
    Ok(())
}

pub(crate) fn atomic_json<T: Serialize>(path: &Path, value: &T) -> Result<(), EngineError> {
    let bytes = serde_json::to_vec(value)?;
    atomic_bytes(path, &bytes)
}

pub(crate) fn atomic_bytes(path: &Path, bytes: &[u8]) -> Result<(), EngineError> {
    let parent = path.parent().ok_or_else(|| {
        EngineError::Storage(format!("{} has no parent directory", path.display()))
    })?;
    ensure_dir(parent)?;
    let file_name = path.file_name().and_then(OsStr::to_str).ok_or_else(|| {
        EngineError::Storage(format!("{} has no UTF-8 file name", path.display()))
    })?;
    let temporary = parent.join(format!(".{file_name}.tmp-{}", std::process::id()));
    let mut options = OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    options.mode(0o600);
    let mut file = options.open(&temporary)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    fs::rename(&temporary, path)?;
    sync_dir(parent)?;
    Ok(())
}

pub(crate) fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T, EngineError> {
    let mut file = File::open(path)?;
    let length = file.metadata()?.len();
    if length > MAX_SNAPSHOT_BYTES.max(MAX_PLAN_BYTES as u64) {
        return Err(EngineError::Corrupt(format!(
            "{} exceeds the read ceiling",
            path.display()
        )));
    }
    let mut bytes = Vec::with_capacity(length as usize);
    file.read_to_end(&mut bytes)?;
    serde_json::from_slice(&bytes)
        .map_err(|error| EngineError::Corrupt(format!("{}: {error}", path.display())))
}

pub(crate) fn snapshot(
    paths: &TxnPaths,
    id: &str,
    faults: &dyn FaultInjector,
) -> Result<SnapshotManifest, EngineError> {
    let txn = paths.transaction(id);
    let snapshot = paths.snapshot(id);
    ensure_dir(&txn)?;
    ensure_dir(&snapshot)?;
    let mut entries = Vec::with_capacity(UciConfig::ALL.len());
    let mut total_bytes = 0_u64;

    for config in UciConfig::ALL {
        faults.check(FaultPoint::Snapshot(config))?;
        let source = paths.config(config);
        let data_path = snapshot.join(format!("{}.data", config.file_name()));
        let missing_path = snapshot.join(format!("{}.missing", config.file_name()));
        if source.exists() {
            let metadata = source.metadata()?;
            if !metadata.is_file() {
                return Err(EngineError::Storage(format!(
                    "allowlisted source {} is not a regular file",
                    source.display()
                )));
            }
            let bytes = fs::read(&source)?;
            total_bytes = total_bytes
                .checked_add(bytes.len() as u64)
                .ok_or(EngineError::SnapshotTooLarge)?;
            if total_bytes > MAX_SNAPSHOT_BYTES {
                return Err(EngineError::SnapshotTooLarge);
            }
            atomic_bytes(&data_path, &bytes)?;
            entries.push(SnapshotEntry {
                config,
                missing: false,
                bytes: bytes.len() as u64,
                digest: Some(blake3::hash(&bytes).to_hex().to_string()),
            });
        } else {
            atomic_bytes(&missing_path, b"missing\n")?;
            entries.push(SnapshotEntry {
                config,
                missing: true,
                bytes: 0,
                digest: None,
            });
        }
    }
    let manifest = SnapshotManifest {
        schema_version: SCHEMA_VERSION,
        entries,
        total_bytes,
    };
    atomic_json(&snapshot.join("manifest.json"), &manifest)?;
    sync_dir(&snapshot)?;
    Ok(manifest)
}

pub(crate) fn validate_snapshot(
    paths: &TxnPaths,
    id: &str,
) -> Result<SnapshotManifest, EngineError> {
    let snapshot = paths.snapshot(id);
    let manifest: SnapshotManifest = read_json(&snapshot.join("manifest.json"))?;
    if manifest.schema_version != SCHEMA_VERSION
        || manifest.entries.len() != UciConfig::ALL.len()
        || manifest.total_bytes > MAX_SNAPSHOT_BYTES
    {
        return Err(EngineError::IncompleteSnapshot);
    }
    for config in UciConfig::ALL {
        let entry = manifest
            .entries
            .iter()
            .find(|entry| entry.config == config)
            .ok_or(EngineError::IncompleteSnapshot)?;
        let data_path = snapshot.join(format!("{}.data", config.file_name()));
        let missing_path = snapshot.join(format!("{}.missing", config.file_name()));
        if entry.missing {
            if !missing_path.is_file() || data_path.exists() {
                return Err(EngineError::IncompleteSnapshot);
            }
        } else {
            if missing_path.exists() || !data_path.is_file() {
                return Err(EngineError::IncompleteSnapshot);
            }
            let bytes = fs::read(data_path)?;
            if entry.bytes != bytes.len() as u64
                || entry.digest.as_deref() != Some(blake3::hash(&bytes).to_hex().as_str())
            {
                return Err(EngineError::IncompleteSnapshot);
            }
        }
    }
    Ok(manifest)
}

pub(crate) fn restore_snapshot(paths: &TxnPaths, id: &str) -> Result<(), EngineError> {
    let manifest = validate_snapshot(paths, id)?;
    ensure_dir(&paths.config_root)?;
    let snapshot = paths.snapshot(id);
    for entry in &manifest.entries {
        let target = paths.config(entry.config);
        if entry.missing {
            match fs::remove_file(&target) {
                Ok(()) => sync_dir(&paths.config_root)?,
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            }
        } else {
            let source = snapshot.join(format!("{}.data", entry.config.file_name()));
            atomic_bytes(&target, &fs::read(source)?)?;
        }
    }
    verify_restored(paths, &manifest)
}

fn verify_restored(paths: &TxnPaths, manifest: &SnapshotManifest) -> Result<(), EngineError> {
    for entry in &manifest.entries {
        let target = paths.config(entry.config);
        if entry.missing {
            if target.exists() {
                return Err(EngineError::RestorationFailed(format!(
                    "{} should be absent",
                    target.display()
                )));
            }
        } else {
            let bytes = fs::read(&target)?;
            if entry.bytes != bytes.len() as u64
                || entry.digest.as_deref() != Some(blake3::hash(&bytes).to_hex().as_str())
            {
                return Err(EngineError::RestorationFailed(format!(
                    "{} did not match its backup",
                    target.display()
                )));
            }
        }
    }
    Ok(())
}

pub(crate) fn remove_active(paths: &TxnPaths, id: &str) -> Result<(), EngineError> {
    let transaction = paths.transaction(id);
    if transaction.exists() {
        fs::remove_dir_all(&transaction)?;
        sync_dir(&paths.active())?;
    }
    if paths.journal().exists() {
        fs::remove_file(paths.journal())?;
        sync_dir(paths.root())?;
    }
    Ok(())
}

pub(crate) fn active_ids(paths: &TxnPaths) -> Result<Vec<String>, EngineError> {
    let mut ids = Vec::new();
    for entry in fs::read_dir(paths.active())? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            ids.push(entry.file_name().to_string_lossy().into_owned());
        } else {
            return Err(EngineError::Corrupt(format!(
                "unexpected file in {}",
                paths.active().display()
            )));
        }
    }
    ids.sort();
    Ok(ids)
}

pub(crate) fn prune_receipts(paths: &TxnPaths) -> Result<(), EngineError> {
    let mut receipts = Vec::new();
    for entry in fs::read_dir(paths.receipts())? {
        let entry = entry?;
        if entry.file_type()?.is_file() {
            let receipt: Receipt = read_json(&entry.path())?;
            receipts.push((receipt.wall_time_ms, entry.path()));
        }
    }
    receipts.sort_by_key(|(wall_time, path)| (*wall_time, path.clone()));
    let excess = receipts.len().saturating_sub(crate::MAX_RECEIPTS);
    for (_, path) in receipts.into_iter().take(excess) {
        fs::remove_file(path)?;
    }
    if excess > 0 {
        sync_dir(&paths.receipts())?;
    }
    Ok(())
}

pub(crate) fn sync_dir(path: &Path) -> Result<(), EngineError> {
    File::open(path)?.sync_all()?;
    Ok(())
}
