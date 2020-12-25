//! File system with directory support
//!
//! Create a filesystem that has a notion of blocks, inodes and directory inodes, by implementing the
//! [`FileSysSupport`], the [`BlockSupport`], the [`InodeSupport`] and the [`DirectorySupport`] traits
//! together (again, all earlier traits are supertraits of the later ones).
//!
//! [`FileSysSupport`]: ../../cplfs_api/fs/trait.FileSysSupport.html
//! [`BlockSupport`]: ../../cplfs_api/fs/trait.BlockSupport.html
//! [`InodeSupport`]: ../../cplfs_api/fs/trait.InodeSupport.html
//! [`DirectorySupport`]: ../../cplfs_api/fs/trait.DirectorySupport.html
//! Make sure this file does not contain any unaddressed `TODO`s anymore when you hand it in.
//!
//! # Status
//!
//! **TODO**: Replace the question mark below with YES, NO, or PARTIAL to
//! indicate the status of this assignment. If you want to tell something
//! about this assignment to the grader, e.g., you have a bug you can't fix,
//! or you want to explain your approach, write it down after the comments
//! section. If you had no major issues and everything works, there is no need to write any comments.
//!
//! COMPLETED: Yes
//!
//! COMMENTS:
//!
//! ...
//!

use crate::b_inode_support::FileSystem;
use crate::helpers::{
    allocate_bitmapregion, allocate_dataregion, allocate_inoderegionblocks, allocate_inodes,
    allocate_rootdirectory, get_direntries, is_valid_dirname, sb_valid, to_char_array, write_dir,
    write_sb,
};
use cplfs_api::fs::{BlockSupport, DirectorySupport, FileSysSupport, InodeSupport};
use cplfs_api::types::{Block, DirEntry, FType, Inode, InodeLike, SuperBlock, DIRNAME_SIZE};

use crate::filesystem_errors::FileSystemError;

use cplfs_api::controller::Device;
use std::path::Path;

/// You are free to choose the name for your file system. As we will use
/// automated tests when grading your assignment, indicate here the name of
/// your file system data type so we can just use `FSName` instead of
/// having to manually figure out the name.
///
///
pub type FSName = FileSystemC;

//region OLD_STRUCTURE

#[derive(Debug)]
/// This is the filesystem structure for this part of the assignment project
pub struct FileSystemC {
    /// We keep a reference to old fs so we can use it's already implmented functions
    pub fs: FileSystem,
}

impl FileSystemC {
    /// This function creates a filesystem_c given an old filesystem
    pub fn create_filesystem(fs: FileSystem) -> FileSystemC {
        FileSystemC { fs }
    }
}

impl FileSysSupport for FileSystemC {
    type Error = FileSystemError;

    fn sb_valid(sb: &SuperBlock) -> bool {
        return sb_valid(sb);
    }

    fn mkfs<P: AsRef<Path>>(path: P, sb: &SuperBlock) -> Result<Self, Self::Error> {
        if !FSName::sb_valid(sb) {
            Err(FileSystemError::InvalidSuperBlock())
        } else {
            let device_result = Device::new(path, sb.block_size, sb.nblocks);

            match device_result {
                Ok(mut device) => {
                    //place superblock at index 0

                    write_sb(sb, &mut device)?;
                    allocate_inoderegionblocks(&sb, &mut device)?;
                    allocate_bitmapregion(&sb, &mut device)?;
                    allocate_dataregion(&sb, &mut device)?;
                    let mut fs_c = FSName::mountfs(device)?;

                    allocate_inodes(&mut fs_c.fs)?;
                    allocate_rootdirectory(&mut fs_c.fs)?;
                    Ok(fs_c)
                }
                Err(e) => Err(FileSystemError::DeviceAPIError(e)),
            }
        }
    }

    fn mountfs(dev: Device) -> Result<Self, Self::Error> {
        match dev.read_block(0) {
            Ok(block) => {
                let sb = &block.deserialize_from::<SuperBlock>(0)?;
                if FSName::sb_valid(sb)
                    && dev.block_size == sb.block_size
                    && dev.nblocks == sb.nblocks
                {
                    let fs = FileSystem::create_filesystem(*sb, Some(dev));
                    let fs_c = FileSystemC::create_filesystem(fs);
                    Ok(fs_c)
                } else {
                    Err(FileSystemError::InvalidSuperBlock())
                }
            }
            Err(e) => Err(FileSystemError::DeviceAPIError(e)),
        }
    }

    fn unmountfs(self) -> Device {
        return self.fs.unmountfs();
    }
}

impl BlockSupport for FileSystemC {
    fn b_get(&self, i: u64) -> Result<Block, Self::Error> {
        return self.fs.b_get(i);
    }

    fn b_put(&mut self, b: &Block) -> Result<(), Self::Error> {
        return self.fs.b_put(b);
    }

    fn b_free(&mut self, i: u64) -> Result<(), Self::Error> {
        return self.fs.b_free(i);
    }

    fn b_zero(&mut self, i: u64) -> Result<(), Self::Error> {
        return self.fs.b_zero(i);
    }

    fn b_alloc(&mut self) -> Result<u64, Self::Error> {
        return self.fs.b_alloc();
    }

    fn sup_get(&self) -> Result<SuperBlock, Self::Error> {
        return self.fs.sup_get();
    }

    fn sup_put(&mut self, sup: &SuperBlock) -> Result<(), Self::Error> {
        return self.fs.sup_put(sup);
    }
}

impl InodeSupport for FileSystemC {
    type Inode = Inode;

    fn i_get(&self, i: u64) -> Result<Self::Inode, Self::Error> {
        return self.fs.i_get(i);
    }

    fn i_put(&mut self, ino: &Self::Inode) -> Result<(), Self::Error> {
        return self.fs.i_put(ino);
    }

    fn i_free(&mut self, i: u64) -> Result<(), Self::Error> {
        return self.fs.i_free(i);
    }

    fn i_alloc(&mut self, ft: FType) -> Result<u64, Self::Error> {
        return self.fs.i_alloc(ft);
    }

    fn i_trunc(&mut self, inode: &mut Self::Inode) -> Result<(), Self::Error> {
        return self.fs.i_trunc(inode);
    }
}

//endregion

//region NEW_STRUCTURE

impl DirectorySupport for FileSystemC {
    fn new_de(inum: u64, name: &str) -> Option<DirEntry> {
        let mut direntry = DirEntry::default();

        if FSName::set_name_str(&mut direntry, name).is_none() {
            return None;
        } else {
            direntry.inum = inum;
        }

        return Some(direntry);
    }

    fn get_name_str(de: &DirEntry) -> String {
        let mut vec = de.name.to_vec();

        let index = vec.iter().position(|&r| r == '\0').unwrap();

        vec.resize(index, '\0');

        let name: String = vec.iter().collect::<String>();

        println!("{}", name);
        return name;
    }

    fn set_name_str(de: &mut DirEntry, name: &str) -> Option<()> {
        if name.len() > 0 && name.len() <= DIRNAME_SIZE {
            if is_valid_dirname(name) {
                match to_char_array(name) {
                    Ok(name) => de.name = name,
                    Err(e) => return None,
                }
                Some(())
            } else {
                None
            }
        } else {
            None
        }
    }

    fn dirlookup(
        &self,
        inode: &Self::Inode,
        name: &str,
    ) -> Result<(Self::Inode, u64), Self::Error> {
        if !is_valid_dirname(name) {
            return Err(FileSystemError::InvalidDirname());
        }
        if inode.get_ft() != FType::TDir {
            return Err(FileSystemError::INodeNotADirectory());
        }

        let dire_entries = get_direntries(&self.fs, inode)?;

        for dir in dire_entries.iter() {
            let dir_name = FSName::get_name_str(&dir.0);
            if dir_name.eq(name) {
                let inode = self.i_get(dir.0.inum)?;
                return Ok((inode, dir.1));
            }
        }
        return Err(FileSystemError::DirectoryNotFound());
    }

    fn dirlink(
        &mut self,
        inode: &mut Self::Inode,
        name: &str,
        inum: u64,
    ) -> Result<u64, Self::Error> {
        if !is_valid_dirname(name) {
            return Err(FileSystemError::InvalidDirname());
        }
        if inode.get_ft() != FType::TDir {
            return Err(FileSystemError::INodeNotADirectory());
        }

        if inum != inode.inum {
            let mut d_inode = self.i_get(inum)?;
            if d_inode.get_ft() == FType::TFree {
                return Err(FileSystemError::INodeNotADirectory());
            }
            d_inode.disk_node.nlink += 1;
            self.i_put(&d_inode)?;
        }
        let dir = &FSName::new_de(inum, name).unwrap();
        let offset = write_dir(&mut self.fs, inode, dir)?;
        return Ok(offset);
    }
}

//endregion

#[cfg(test)]
#[path = "../../api/fs-tests"]
mod test_with_utils {
    use super::FSName;
    use cplfs_api::fs::{DirectorySupport, FileSysSupport};
    use cplfs_api::types::SuperBlock;
    use std::path::PathBuf;

    #[path = "utils.rs"]
    mod utils;

    static BLOCK_SIZE: u64 = 1000;
    static NBLOCKS: u64 = 10;
    static SUPERBLOCK_GOOD: SuperBlock = SuperBlock {
        block_size: BLOCK_SIZE,
        nblocks: NBLOCKS,
        ninodes: 8,
        inodestart: 1,
        ndatablocks: 5,
        bmapstart: 4,
        datastart: 5,
    };

    #[test]
    fn unit_test() {
        let path = disk_prep_path("mkfs");
        let myfs = FSName::mkfs(&path, &SUPERBLOCK_GOOD).unwrap();

        let name1 = "test.:d"; //should stop reading at the end string char
        assert!(FSName::new_de(0, name1).is_none());

        let name2 = "tes.t."; //should stop reading at the end string char
        let de = FSName::new_de(0, name2).unwrap();
        assert_eq!("tes.t.", FSName::get_name_str(&de));
    }

    fn disk_prep_path(name: &str) -> PathBuf {
        utils::disk_prep_path(&("fs-images-c-".to_string() + name), "img")
    }
}

// WARNING: DO NOT TOUCH THE BELOW CODE -- IT IS REQUIRED FOR TESTING -- YOU WILL LOSE POINTS IF I MANUALLY HAVE TO FIX YOUR TESTS
#[cfg(all(test, any(feature = "c", feature = "all")))]
#[path = "../../api/fs-tests/c_test.rs"]
mod tests;
