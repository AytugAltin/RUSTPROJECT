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
//! COMPLETED: ?
//!
//! COMMENTS:
//!
//! ...
//!

use crate::a_block_support::FileSystem;
use cplfs_api::fs::{DirectorySupport, InodeSupport, FileSysSupport, BlockSupport};
use cplfs_api::types::{DirEntry, DIRNAME_SIZE, InodeLike, FType};
use crate::helpers::{to_char_array, is_valid_dirname, get_inode_block_size, get_direntries, write_block, write_dir};
use std::fs::File;
use crate::filesystem_errors::FileSystemError;
use std::convert::TryInto;
use std::ops::Index;

/// You are free to choose the name for your file system. As we will use
/// automated tests when grading your assignment, indicate here the name of
/// your file system data type so we can just use `FSName` instead of
/// having to manually figure out the name.
/// **TODO**: replace the below type by the type of your file system
pub type FSName = FileSystem;

impl DirectorySupport for FileSystem {

    fn new_de(inum: u64, name: &str) -> Option<DirEntry> {
        let mut direntry =  DirEntry::default();

        if FileSystem::set_name_str(&mut direntry, name).is_none(){
            return None
        }
        else{
            direntry.inum = inum;
        }

        return Some(direntry)
    }

    fn get_name_str(de: &DirEntry) -> String {
        let mut vec = de.name.to_vec();

        let index = vec.iter().position(|&r| r == '\0').unwrap();

        vec.resize(index,'\0');

        let name:String =  vec.iter().collect::<String>();

        println!("{}",name);
        return name;
    }

    fn set_name_str(de: &mut DirEntry, name: &str) -> Option<()> {
        if name.len() > 0 && name.len() <= DIRNAME_SIZE {
            if is_valid_dirname(name) {
                match to_char_array(name) {
                    Ok(name) =>

                        de.name = name
                    ,
                    Err(e) => return None
                }
                Some(())
            }
            else{
                None
            }
        }
        else{
            None
        }
    }

    fn dirlookup(&self, inode: &Self::Inode, name: &str) -> Result<(Self::Inode, u64), Self::Error> {

        let mut dire_entries = get_direntries(self,inode)?;

        for dir in dire_entries.iter(){
            let dir_name = FSName::get_name_str(&dir.0);
            if dir_name.eq(name) {
                let inode = self.i_get(dir.0.inum)?;
                return Ok((inode,dir.1))
            }
        }

        unimplemented!()

    }

    fn dirlink(&mut self, inode: &mut Self::Inode, name: &str, inum: u64) -> Result<u64, Self::Error> {
        if is_valid_dirname(name) && inode.get_ft() == FType::TDir {
            let fs_inode = self.i_get(inum)?;

            let dir = &FSName::new_de(inum,name).unwrap();

            write_dir(self,inode,dir);

        }
        unimplemented!()
    }
}

// **TODO** define your own tests here.

#[cfg(test)]
#[path = "../../api/fs-tests"]
mod test_with_utils {
    use super::FSName;
    use cplfs_api::fs::{BlockSupport, DirectorySupport, FileSysSupport, InodeSupport};
    use cplfs_api::types::{FType, InodeLike, SuperBlock, DIRENTRY_SIZE};
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
        let my_fs = FSName::mkfs(&path, &SUPERBLOCK_GOOD).unwrap();

        let name1 = "test.:d"; //should stop reading at the end string char
        assert!(FSName::new_de(0, name1).is_none());

        let name2 = "tes.t."; //should stop reading at the end string char
        let mut de = FSName::new_de(0, name2).unwrap();
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
