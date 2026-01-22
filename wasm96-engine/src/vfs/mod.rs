//! wasm96 Virtual File System (VFS).
//!
//! This module provides a FAT-formatted filesystem that can be either in-memory
//! or backed by a persistent storage (like a `.img` file or libretro SRAM).
//! It uses `fatfs` for the filesystem logic.

use anyhow::Result;
use std::io::Cursor;
use std::path::Path;
use std::sync::{Arc, Mutex};

/// A handle to a virtual disk.
pub struct VirtualDisk {
    inner: Arc<Mutex<Cursor<Vec<u8>>>>,
}

impl VirtualDisk {
    /// Create a new in-memory disk of a fixed size.
    pub fn new_in_memory(size: usize) -> Self {
        let buffer = vec![0u8; size];
        Self {
            inner: Arc::new(Mutex::new(Cursor::new(buffer))),
        }
    }

    /// Create a disk from an existing byte buffer (e.g., loaded from a file or SRAM).
    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        Self {
            inner: Arc::new(Mutex::new(Cursor::new(bytes))),
        }
    }

    /// Export the disk as a byte vector (for saving to file or SRAM).
    pub fn export(&self) -> Vec<u8> {
        let lock = self.inner.lock().unwrap();
        lock.get_ref().clone()
    }

    /// Formats the disk with a FAT filesystem.
    /// This should be called if the disk is freshly created and not already formatted.
    pub fn format(&self, label: &str) -> Result<()> {
        let mut lock = self.inner.lock().unwrap();
        let cursor = &mut *lock;
        cursor.set_position(0);

        // FAT labels are 11 characters
        let mut label_bytes = [b' '; 11];
        let bytes = label.as_bytes();
        let len = bytes.len().min(11);
        label_bytes[..len].copy_from_slice(&bytes[..len]);

        let options = fatfs::FormatVolumeOptions::new().volume_label(label_bytes);

        fatfs::format_volume(cursor, options)?;
        Ok(())
    }

    /// Read a file's contents from the virtual disk.
    pub fn read_file(&self, path: &str) -> Result<Vec<u8>> {
        let mut lock = self.inner.lock().unwrap();
        let cursor = &mut *lock;
        cursor.set_position(0);
        let fs = fatfs::FileSystem::new(cursor, fatfs::FsOptions::new())?;
        let mut file = fs.root_dir().open_file(path)?;
        let mut contents = Vec::new();
        use std::io::Read;
        file.read_to_end(&mut contents)?;
        Ok(contents)
    }

    /// Write a file's contents to the virtual disk.
    pub fn write_file(&self, path: &str, data: &[u8]) -> Result<()> {
        let mut lock = self.inner.lock().unwrap();
        let cursor = &mut *lock;
        cursor.set_position(0);
        let fs = fatfs::FileSystem::new(cursor, fatfs::FsOptions::new())?;
        let mut file = fs.root_dir().create_file(path)?;
        use std::io::Write;
        file.write_all(data)?;
        Ok(())
    }

    /// Pack a host directory into the virtual disk.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn pack_from_host(&self, host_path: &Path) -> Result<()> {
        let mut lock = self.inner.lock().unwrap();
        let cursor = &mut *lock;
        cursor.set_position(0);
        let fs = fatfs::FileSystem::new(cursor, fatfs::FsOptions::new())?;
        let root = fs.root_dir();
        Self::pack_recursive(&root, host_path)?;
        Ok(())
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn pack_recursive<IO: fatfs::ReadWriteSeek>(
        vdir: &fatfs::Dir<IO>,
        host_path: &Path,
    ) -> Result<()> {
        for entry in std::fs::read_dir(host_path)? {
            let entry = entry?;
            let path = entry.path();
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .ok_or_else(|| anyhow::anyhow!("Invalid filename"))?;

            if path.is_dir() {
                let new_vdir = vdir.create_dir(name)?;
                Self::pack_recursive(&new_vdir, &path)?;
            } else {
                let mut vfile = vdir.create_file(name)?;
                let mut hfile = std::fs::File::open(path)?;
                std::io::copy(&mut hfile, &mut vfile)?;
            }
        }
        Ok(())
    }

    /// Extract the virtual disk contents to a host directory.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn extract_to_host(&self, host_path: &Path) -> Result<()> {
        if !host_path.exists() {
            std::fs::create_dir_all(host_path)?;
        }
        let mut lock = self.inner.lock().unwrap();
        let cursor = &mut *lock;
        cursor.set_position(0);
        let fs = fatfs::FileSystem::new(cursor, fatfs::FsOptions::new())?;
        let root = fs.root_dir();
        Self::extract_recursive(&root, host_path)?;
        Ok(())
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn extract_recursive<IO: fatfs::ReadWriteSeek>(
        vdir: &fatfs::Dir<IO>,
        host_path: &Path,
    ) -> Result<()> {
        for entry_res in vdir.iter() {
            let entry = entry_res?;
            let name = entry.file_name();
            if name == "." || name == ".." {
                continue;
            }
            let path = host_path.join(&name);
            if entry.is_dir() {
                std::fs::create_dir_all(&path)?;
                let next_vdir = entry.to_dir();
                Self::extract_recursive(&next_vdir, &path)?;
            } else {
                let mut vfile = entry.to_file();
                let mut hfile = std::fs::File::create(path)?;
                std::io::copy(&mut vfile, &mut hfile)?;
            }
        }
        Ok(())
    }
}

/// The global VFS state managed by the engine.
pub struct VfsState {
    pub disks: [Option<VirtualDisk>; 5],
}

impl Default for VfsState {
    fn default() -> Self {
        Self {
            disks: [None, None, None, None, None],
        }
    }
}

impl VfsState {
    /// Get a reference to the primary disk (DISK0).
    pub fn disk(&self) -> Option<&VirtualDisk> {
        self.disks[0].as_ref()
    }

    /// Mount a disk into a specific slot (0-4).
    pub fn mount_slot(&mut self, slot: usize, disk: VirtualDisk) {
        if slot < self.disks.len() {
            self.disks[slot] = Some(disk);
        }
    }

    /// Mount a disk into the first available slot, or DISK0 if all full.
    pub fn mount(&mut self, disk: VirtualDisk) {
        for slot in 0..self.disks.len() {
            if self.disks[slot].is_none() {
                self.disks[slot] = Some(disk);
                return;
            }
        }
        self.disks[0] = Some(disk);
    }

    /// Initialize a default 4MB in-memory disk in DISK0 if none exists.
    pub fn ensure_initialized(&mut self) {
        if self.disks[0].is_none() {
            let disk = VirtualDisk::new_in_memory(4 * 1024 * 1024); // 4MB default
            let _ = disk.format("WASM96");
            self.disks[0] = Some(disk);
        }
    }
}
