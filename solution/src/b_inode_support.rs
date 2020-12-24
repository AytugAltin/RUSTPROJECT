//! File system with inode support
//!
//! Create a filesystem that has a notion of inodes and blocks, by implementing the [`FileSysSupport`],
//! the [`BlockSupport`] and the [`InodeSupport`] traits together (again, all earlier traits are
//! supertraits of the later ones).
//!
//! [`FileSysSupport`]: ../../cplfs_api/fs/trait.FileSysSupport.html
//! [`BlockSupport`]: ../../cplfs_api/fs/trait.BlockSupport.html
//! [`InodeSupport`]: ../../cplfs_api/fs/trait.InodeSupport.html
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
use cplfs_api::fs::{InodeSupport, FileSysSupport, BlockSupport};
use cplfs_api::types::{FType, Inode, InodeLike, DINODE_SIZE, DInode};
use crate::filesystem_errors::FileSystemError::IndexOutOfBounds;
use crate::helpers::{get_inode_block, trunc};
use crate::filesystem_errors::FileSystemError;
use std::ptr::eq;
use std::borrow::BorrowMut;


/// You are free to choose the name for your file system. As we will use
/// automated tests when grading your assignment, indicate here the name of
/// your file system data type so we can just use `FSName` instead of
/// having to manually figure out the name.
/// **TODO**: replace the below type by the type of your file system
pub type FSName = FileSystem;

impl InodeSupport for FileSystem {
    type Inode = Inode;

    fn i_get(&self, i: u64) -> Result<Self::Inode, Self::Error> {

        let inodes_per_block = self.superblock.block_size / *DINODE_SIZE;
        let mut block = get_inode_block(self, i,inodes_per_block)?;
        let block_inode_offset = i % inodes_per_block * *DINODE_SIZE;

        let disk_node = block.deserialize_from::<DInode>(block_inode_offset)?;

        return Ok(Inode::new(i, disk_node));
    }


    fn i_put(&mut self, ino: &Self::Inode) -> Result<(), Self::Error> {

        let inodes_per_block = self.superblock.block_size / *DINODE_SIZE;
        let mut block = get_inode_block(self, ino.inum,inodes_per_block)?;

        let block_inode_offset = ino.inum % inodes_per_block * *DINODE_SIZE;

        block.serialize_into(&ino.disk_node,block_inode_offset)?;

        self.b_put(&block)?;

        Ok(())
    }

    fn i_free(&mut self, i: u64) -> Result<(), Self::Error> {
        let mut ino = self.i_get(i)?;

        if ino.disk_node.nlink == 0 && ino.inum > 0{
            trunc(self, ino.borrow_mut())?;
            ino.disk_node.ft = FType::TFree;
            self.i_put(&ino);
            Ok(())
        }
        else
        {
            Err(FileSystemError::INodeNotFreeable())
        }
    }

    fn i_alloc(&mut self, ft: FType) -> Result<u64, Self::Error> {
        let inode_alloc_start = 1;
        for i in inode_alloc_start..self.superblock.ninodes{
            let mut ino = self.i_get(i)?;
            if ino.get_ft() == FType::TFree{
                ino.disk_node.ft = ft;
                self.i_put(&ino);
                return Ok(i)
            }
        }
        return Err(FileSystemError::AllocationError())
    }

    fn i_trunc(&mut self, inode: &mut Self::Inode) -> Result<(), Self::Error> {
        //TODO DO i need to raise error when these are not equal
        let mut ino = self.i_get(inode.inum)?;
        if &ino == inode{
            trunc(self,inode)?;
            self.i_put(&inode);
        }
        else{
        }
        Ok(())
    }
}





// **TODO** define your own tests here.

// WARNING: DO NOT TOUCH THE BELOW CODE -- IT IS REQUIRED FOR TESTING -- YOU WILL LOSE POINTS IF I MANUALLY HAVE TO FIX YOUR TESTS
#[cfg(all(test, any(feature = "b", feature = "all")))]
#[path = "../../api/fs-tests/b_test.rs"]
mod tests;
