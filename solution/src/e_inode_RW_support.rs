//! File system with inode support + read and write operations on inodes
//!
//! Create a filesystem that has a notion of inodes and blocks, by implementing the [`FileSysSupport`], the [`BlockSupport`] and the [`InodeSupport`] traits together (again, all earlier traits are supertraits of the later ones).
//! Additionally, implement the [`InodeRWSupport`] trait to provide operations to read from and write to inodes
//!
//! [`FileSysSupport`]: ../../cplfs_api/fs/trait.FileSysSupport.html
//! [`BlockSupport`]: ../../cplfs_api/fs/trait.BlockSupport.html
//! [`InodeSupport`]: ../../cplfs_api/fs/trait.InodeSupport.html
//! [`InodeRWSupport`]: ../../cplfs_api/fs/trait.InodeRWSupport.html
//! Make sure this file does not contain any unaddressed
//!
//! # Status
//!
//!
//! indicate the status of this assignment. If you want to tell something
//! about this assignment to the grader, e.g., you have a bug you can't fix,
//! or you want to explain your approach, write it down after the comments
//! section. If you had no major issues and everything works, there is no need to write any comments.
//!
//! COMPLETED: Yes
//!
//! COMMENTS: I could have tested a little bit more
//!
//! ...
//!

use crate::c_dirs_support::FileSystemC;
use crate::filesystem_errors::FileSystemError;
use cplfs_api::fs::{BlockSupport, InodeRWSupport, InodeSupport};
use cplfs_api::types::{Buffer, InodeLike};
use std::convert::TryFrom;

/// You are free to choose the name for your file system. As we will use
/// automated tests when grading your assignment, indicate here the name of
/// your file system data type so we can just use `FSName` instead of
/// having to manually figure out the name.
///
pub type FSName = FileSystemC;

impl InodeRWSupport for FileSystemC {

    fn i_read(&self,inode: &Self::Inode,buf: &mut Buffer,off: u64,n: u64 ) -> Result<u64, Self::Error> {
        let mut ofsset = off;

        let mut length = n;
        let mut list = Vec::new();
        let mut bytes_read = 0;

        if ofsset > u64::try_from(inode.get_size()).unwrap(){
            return Err(FileSystemError::ReadError());// WRITE EERROR
        }
        if ofsset == inode.get_size() {
            return Ok(0)
        }

        for i in inode.disk_node.direct_blocks.iter() {
            if *i != 0 && length > 0 {
                if ofsset >= self.fs.superblock.block_size {
                    ofsset -= self.fs.superblock.block_size;
                } else {
                    let block = self.fs.b_get(*i)?;
                    let  contents_ref;
                    contents_ref = block.contents_as_ref();

                    if length + ofsset > self.fs.superblock.block_size {
                        let start = usize::try_from(ofsset).unwrap();
                        let end = usize::try_from(self.fs.superblock.block_size).unwrap();
                        list.extend_from_slice(&contents_ref[start..end]);
                        bytes_read = bytes_read + self.fs.superblock.block_size - ofsset;
                        length = length - (self.fs.superblock.block_size - ofsset);
                    } else {
                        let start = usize::try_from(ofsset).unwrap();
                        let end = usize::try_from(ofsset + length).unwrap();
                        list.extend_from_slice(&contents_ref[start..end]);
                        //bytes_read = bytes_read + length - ofsset;
                        bytes_read = bytes_read + u64::try_from(end-start).unwrap();
                        length = 0;
                    }
                    ofsset = 0;
                }
            }
        }
        let limit = usize::try_from(buf.len()).unwrap();

        if limit < list.len() {
            buf.write_data(&list.as_slice()[0..limit], 0)?;
        } else {
            buf.write_data(&list.as_slice(), 0)?;
        }
        return Ok(bytes_read);
    }

    fn i_write(&mut self,inode: &mut Self::Inode,buf: &Buffer,off: u64,n: u64,) -> Result<(), Self::Error> {
        let mut ofsset = off;

        let mut towrite_length:usize = usize::try_from(n).unwrap();

        let mut vector = buf.contents_as_ref().to_vec();

        if ofsset > u64::try_from(inode.get_size()).unwrap(){
            return Err(FileSystemError::ReadError());// WRITE EERROR
        }
        if n > buf.len(){
            return Err(FileSystemError::ReadError());// WRITE EERROR
        }

        if n == 0 {
            return Ok(())
        }

        let mut potential_size = 0;

        for i in 0..inode.disk_node.direct_blocks.len() {
            if inode.disk_node.direct_blocks[i] != 0{
            potential_size += self.fs.superblock.block_size;
            }
        }


        if potential_size < ofsset+n {
            // need a new block
            for i in 0..inode.disk_node.direct_blocks.len() {
                if inode.disk_node.direct_blocks[i] == 0 && potential_size < ofsset+n{
                    // found a new block
                    let mut new_block_data_index = self.b_alloc()?;
                    let block_index = new_block_data_index + self.fs.superblock.datastart;
                    inode.disk_node.direct_blocks[i] = block_index;

                    if potential_size + self.fs.superblock.block_size < ofsset+n {
                        inode.disk_node.size += self.fs.superblock.block_size;
                        potential_size += self.fs.superblock.block_size;
                    }
                    else{
                        inode.disk_node.size += (ofsset+n) - inode.get_size();
                        potential_size += self.fs.superblock.block_size;
                    }
                }
            }
        }
        else{
            inode.disk_node.size += (ofsset+n) - inode.get_size();
        }

        if inode.get_size() < ofsset+n {
            return Err(FileSystemError::AllocationError());// no room; inode is full
        }



        for i in inode.disk_node.direct_blocks.iter() {
            if *i != 0 && towrite_length > 0 {
                if ofsset >= self.fs.superblock.block_size {
                    ofsset -= self.fs.superblock.block_size;// GET TO THE RIGHT BLOCK
                }
                else
                {
                    let mut block = self.fs.b_get(*i)?;

                    let mut block_space = usize::try_from(block.len() - ofsset).unwrap();

                    if block_space > towrite_length{
                        let mut temp = vector.get(0..towrite_length).unwrap();
                        block.write_data(temp,ofsset)?;
                        self.b_put(&block);
                        self.i_put(inode);
                        return Ok(())
                    }
                    else{
                        let mut temp = vector.get(0..block_space).unwrap();
                        block.write_data(temp,ofsset)?;
                        vector = vector.split_off(block_space).to_vec();

                        towrite_length = towrite_length - block_space;
                        self.b_put(&block);
                        self.i_put(inode);
                        ofsset = 0;
                    }

                }
            }
        }

        return Err(FileSystemError::AllocationError());// no room; inode is full
    }
}

//
// WARNING: DO NOT TOUCH THE BELOW CODE -- IT IS REQUIRED FOR TESTING -- YOU WILL LOSE POINTS IF I MANUALLY HAVE TO FIX YOUR TESTS
#[cfg(all(test, any(feature = "e", feature = "all")))]
#[path = "../../api/fs-tests/e_test.rs"]
mod tests;
