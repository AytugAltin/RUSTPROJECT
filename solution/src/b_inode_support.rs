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
//! COMPLETED: YES
//!
//! COMMENTS:
//!
//!

//use crate::a_block_support::FileSystem;
use cplfs_api::fs::{BlockSupport, FileSysSupport, InodeSupport};
use cplfs_api::types::{DInode, FType, Inode, InodeLike, DINODE_SIZE};

use crate::filesystem_errors::FileSystemError;
use crate::helpers::{get_inode_block, trunc};

use std::borrow::BorrowMut;

use cplfs_api::controller::Device;
use cplfs_api::types::{Block, SuperBlock};
use std::path::Path;

use crate::helpers::*;

/// You are free to choose the name for your file system. As we will use
/// automated tests when grading your assignment, indicate here the name of
/// your file system data type so we can just use `FSName` instead of
/// having to manually figure out the name.
/// *
pub type FSName = FileSystem;

#[derive(Debug)]
/// This is the filesystem structure that wa are going to use in the whole project
pub struct FileSystem {
    /// We keep a reference to the superblock cause it can come in hand
    pub superblock: SuperBlock,
    /// This is the device we work on, it is optional at the start and can be filled in later
    pub device: Option<Device>,
}

impl FileSystem {
    /// This function creates a filesystem struct given a superblock and a optional device
    pub fn create_filesystem(superblock: SuperBlock, device: Option<Device>) -> FileSystem {
        FileSystem { superblock, device }
    }
}

impl FileSysSupport for FileSystem {
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
                    let mut fs = FileSystem::mountfs(device)?;

                    allocate_inodes(&mut fs)?;
                    Ok(fs)
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
                    Ok(fs)
                } else {
                    Err(FileSystemError::InvalidSuperBlock())
                }
            }
            Err(e) => Err(FileSystemError::DeviceAPIError(e)),
        }
    }

    fn unmountfs(mut self) -> Device {
        let deviceoption = self.device.take();
        let device = deviceoption.unwrap();
        return device;
    }
}

impl BlockSupport for FileSystem {
    fn b_get(&self, i: u64) -> Result<Block, Self::Error> {
        let dev = self
            .device
            .as_ref()
            .ok_or_else(|| FileSystemError::DeviceNotSet())?;
        return Ok(read_block(dev, i)?);
    }

    fn b_put(&mut self, b: &Block) -> Result<(), Self::Error> {
        let dev = self
            .device
            .as_mut()
            .ok_or_else(|| FileSystemError::DeviceNotSet())?;
        return Ok(write_block(dev, b)?);
    }

    fn b_free(&mut self, i: u64) -> Result<(), Self::Error> {
        let dev = self
            .device
            .as_mut()
            .ok_or_else(|| FileSystemError::DeviceNotSet())?;
        set_bitmapbit(&self.superblock, dev, i, false)?;
        Ok(())
    }

    fn b_zero(&mut self, i: u64) -> Result<(), Self::Error> {
        let datablock_index = i + self.superblock.datastart;
        let newzeroblock = Block::new(
            datablock_index,
            vec![0; self.superblock.block_size as usize].into_boxed_slice(),
        ); //TODO last change
        self.b_put(&newzeroblock)?;
        Ok(())
    }

    fn b_alloc(&mut self) -> Result<u64, Self::Error> {
        let nbitmapblocks = get_nbitmapblocks(&self.superblock);
        let mut bmstart_index = self.superblock.bmapstart; // get the index
        let mut block; // get the first block
        let mut byte_array; // create an empty data buffer
        let mut byteindex; //block index

        for blockindex in 0..nbitmapblocks {
            block = self.b_get(bmstart_index + blockindex)?; //next block
                                                             //byte_array = block.contents_as_ref(); //get the block's array
            byte_array = block.contents_as_ref();
            byteindex = get_bytesarray_free_index(byte_array);
            if byteindex.is_err() {
                // HERE WE ARE LOOKING FOR THE NEXT BLOCK
                bmstart_index += 1; //next block index
            } else {
                // The current bm_block has a free spot
                let byteindex = byteindex.unwrap(); //get the index of the byte that has a free spot
                let byte = byte_array.get(usize::from(byteindex)).unwrap();
                let bitindex = 8 - 1 - byte.trailing_ones(); //moves the 1 to the correct position
                let mutator = 0b00000001u8 << byte.trailing_ones(); //moves the 1 to the correct position

                let to_write_byte = &[(*byte | mutator)];

                block.write_data(to_write_byte, byteindex as u64)?;

                let byteindex: u64 = u64::from(byteindex);
                let datablockindex = blockindex * self.superblock.block_size * 8
                    + (byteindex) * 8
                    + u64::from(7 - bitindex);

                if datablockindex < self.superblock.ndatablocks {
                    self.b_zero(datablockindex)?;
                    self.b_put(&block)?;
                    return Ok(datablockindex);
                }
            }
        }
        Err(FileSystemError::AllocationError())
    }

    fn sup_get(&self) -> Result<SuperBlock, Self::Error> {
        let block = self.b_get(0)?;
        let sb = block.deserialize_from::<SuperBlock>(0)?;
        Ok(sb)
    }

    fn sup_put(&mut self, sup: &SuperBlock) -> Result<(), Self::Error> {
        let mut firstblock = self.b_get(0)?;
        firstblock.serialize_into(&sup, 0)?;
        self.b_put(&firstblock)?;
        Ok(()) //TODO check return type, is this oke to do or not?
    }
}

impl InodeSupport for FileSystem {
    type Inode = Inode;

    fn i_get(&self, i: u64) -> Result<Self::Inode, Self::Error> {
        let inodes_per_block = self.superblock.block_size / *DINODE_SIZE;
        let block = get_inode_block(self, i, inodes_per_block)?;
        let block_inode_offset = i % inodes_per_block * *DINODE_SIZE;

        let disk_node = block.deserialize_from::<DInode>(block_inode_offset)?;

        return Ok(Inode::new(i, disk_node));
    }

    fn i_put(&mut self, ino: &Self::Inode) -> Result<(), Self::Error> {
        let inodes_per_block = self.superblock.block_size / *DINODE_SIZE;
        let mut block = get_inode_block(self, ino.inum, inodes_per_block)?;

        let block_inode_offset = ino.inum % inodes_per_block * *DINODE_SIZE;

        block.serialize_into(&ino.disk_node, block_inode_offset)?;

        self.b_put(&block)?;

        Ok(())
    }

    fn i_free(&mut self, i: u64) -> Result<(), Self::Error> {
        let mut ino = self.i_get(i)?;

        if ino.disk_node.nlink == 0 && ino.inum > 0 {
            trunc(self, ino.borrow_mut())?;
            ino.disk_node.ft = FType::TFree;
            self.i_put(&ino)?;
            Ok(())
        } else {
            Err(FileSystemError::INodeNotFreeable())
        }
    }

    fn i_alloc(&mut self, ft: FType) -> Result<u64, Self::Error> {
        let inode_alloc_start = 1;
        for i in inode_alloc_start..self.superblock.ninodes {
            let mut ino = self.i_get(i)?;
            if ino.get_ft() == FType::TFree {
                ino.disk_node.ft = ft;
                self.i_put(&ino)?;
                return Ok(i);
            }
        }
        return Err(FileSystemError::AllocationError());
    }

    fn i_trunc(&mut self, inode: &mut Self::Inode) -> Result<(), Self::Error> {
        //TODO DO i need to raise error when these are not equal
        let ino = self.i_get(inode.inum)?;

        if &ino == inode {
            trunc(self, inode)?;
            self.i_put(&inode)?;
        } else {
        }

        Ok(())
    }
}

// **TODO** define your own tests here.

// WARNING: DO NOT TOUCH THE BELOW CODE -- IT IS REQUIRED FOR TESTING -- YOU WILL LOSE POINTS IF I MANUALLY HAVE TO FIX YOUR TESTS
#[cfg(all(test, any(feature = "b", feature = "all")))]
#[path = "../../api/fs-tests/b_test.rs"]
mod tests;
