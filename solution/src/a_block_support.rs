//! File system with block support
//!
//! Create a filesystem that only has a notion of blocks, by implementing the [`FileSysSupport`] and
//! the [`BlockSupport`] traits together (you have no other choice, as the first one is a supertrait of the second).
//!
//! [`FileSysSupport`]: ../../cplfs_api/fs/trait.FileSysSupport.html
//! [`BlockSupport`]: ../../cplfs_api/fs/trait.BlockSupport.html
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
//! COMMENTS: I realized to late that i could have used the sup_get function when wanting to use the superblock,
//! Instead i kept the parameter superblock in the structure
//!
//! ...
//!

// Turn off the warnings we get from the below example imports, which are currently unused.

// If you want to import things from the API crate, do so as follows:
use cplfs_api::controller::Device;
use cplfs_api::fs::{BlockSupport, FileSysSupport};
use cplfs_api::types::{Block, SuperBlock};
use std::path::Path;

use crate::filesystem_errors::FileSystemError;

use crate::helpers::*;

/// You are free to choose the name for your file system. As we will use
/// automated tests when grading your assignment, indicate here the name of
/// your file system data type so we can just use `FSName` instead of
/// having to manually figure out your file system name.
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
                    let fs = FileSystem::mountfs(device)?;

                    //allocate_inodes(&mut fs);
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

// Here we define a submodule, called `my_tests`, that will contain your unit
// tests for this module.
// You can define more tests in different modules, and change the name of this module
//
// The `test` in the `#[cfg(test)]` annotation ensures that this code is only compiled when we're testing the code.
// To run these tests, run the command `cargo test` in the `solution` directory
//
// To learn more about testing, check the Testing chapter of the Rust
// Book: https://doc.rust-lang.org/book/testing.html
#[cfg(test)]
mod superblock_tests {

    use super::FSName;

    use cplfs_api::fs::FileSysSupport;
    use cplfs_api::types::SuperBlock;

    #[path = "utils.rs"]
    #[test]
    fn trivial_unit_test() {
        assert_eq!(FSName::sb_valid(&SUPERBLOCK_OVERSIZED), false);
        assert_eq!(FSName::sb_valid(&SUPERBLOCK_GOOD), true);
        assert_eq!(FSName::sb_valid(&SUPERBLOCK_BAD_1), true);
        assert_eq!(FSName::sb_valid(&SUPERBLOCK_BAD_2), true);
        assert_eq!(FSName::sb_valid(&SUPERBLOCK_BAD_3), true);
    }

    static BLOCK_SIZE: u64 = 1000;
    static NBLOCKS: u64 = 10;

    static SUPERBLOCK_OVERSIZED: SuperBlock = SuperBlock {
        block_size: BLOCK_SIZE,
        nblocks: NBLOCKS,
        ninodes: 1000,
        inodestart: 1,
        ndatablocks: 5,
        bmapstart: 5,
        datastart: 6,
    };

    static SUPERBLOCK_GOOD: SuperBlock = SuperBlock {
        block_size: BLOCK_SIZE,
        nblocks: NBLOCKS,
        ninodes: 6,
        inodestart: 1,
        ndatablocks: 4,
        bmapstart: 5,
        datastart: 6,
    };

    static SUPERBLOCK_BAD_1: SuperBlock = SuperBlock {
        block_size: BLOCK_SIZE,
        nblocks: NBLOCKS,
        ninodes: 6,
        inodestart: 1,
        ndatablocks: 2,
        bmapstart: 5,
        datastart: 6,
    };

    static SUPERBLOCK_BAD_2: SuperBlock = SuperBlock {
        block_size: BLOCK_SIZE,
        nblocks: NBLOCKS,
        ninodes: 1,
        inodestart: 1,
        ndatablocks: 5,
        bmapstart: 5,
        datastart: 6,
    };

    static SUPERBLOCK_BAD_3: SuperBlock = SuperBlock {
        block_size: BLOCK_SIZE,
        nblocks: NBLOCKS,
        ninodes: 1,
        inodestart: 1,
        ndatablocks: 5,
        bmapstart: 4,
        datastart: 5,
    };
}

// If you want to write more complicated tests that create actual files on your system, take a look at `utils.rs` in the assignment, and how it is used in the `fs_tests` folder to perform the tests. I have imported it below to show you how it can be used.
// The `utils` folder has a few other useful methods too (nothing too crazy though, you might want to write your own utility functions, or use a testing framework in rust, if you want more advanced features)
#[cfg(test)]
#[path = "../../api/fs-tests"]
mod test_with_utils {

    use crate::a_block_support::FSName;

    use cplfs_api::controller::Device;
    use cplfs_api::fs::{BlockSupport, FileSysSupport};
    use cplfs_api::types::SuperBlock;
    use std::path::{Path, PathBuf};

    #[path = "utils.rs"]
    mod utils;

    static BLOCK_SIZE: u64 = 1000;
    static NBLOCKS: u64 = 10;
    static SUPERBLOCK_GOOD: SuperBlock = SuperBlock {
        block_size: BLOCK_SIZE,
        nblocks: NBLOCKS,
        ninodes: 6,
        inodestart: 1,
        ndatablocks: 4,
        bmapstart: 5,
        datastart: 6,
    };

    fn disk_prep_path(name: &str) -> PathBuf {
        utils::disk_prep_path(&("fs-images-a-".to_string() + name), "img")
    }

    //Create a fresh device
    fn disk_setup(path: &Path) -> Device {
        utils::disk_setup(path, BLOCK_SIZE, NBLOCKS)
    }

    #[test]
    fn complex_test() {
        let path = disk_prep_path("test");
        let mut my_fs = FSName::mkfs(&path, &SUPERBLOCK_GOOD).unwrap();

        let zb = |i| utils::zero_block(i, BLOCK_SIZE);
        for i in 1..NBLOCKS {
            //Will fail if you sneak in inodesupport
            assert_eq!(my_fs.b_get(i).unwrap(), zb(i));
        }

        let nb = utils::n_block(5, BLOCK_SIZE, 6);
        my_fs.b_put(&nb).unwrap();

        let b = my_fs.b_get(5).unwrap();
        assert_eq!(b, nb);

        let nb_bis = utils::n_block(6, BLOCK_SIZE, 6);
        my_fs.b_put(&nb_bis).unwrap();
        my_fs.b_zero(1).unwrap(); //zero this block again
        let b = my_fs.b_get(6).unwrap();

        assert_eq!(b, zb(6));

        assert!(my_fs.b_zero(5).is_err()); //out of bounds

        /*
               for i in 0..SUPERBLOCK_GOOD.ndatablocks {
                   assert!(my_fs.b_alloc().is_err());
               }

        */

        let dev = my_fs.unmountfs();
        utils::disk_destruct(dev);
    }
}

// Here we define a submodule, called `tests`, that will contain our unit tests
// Take a look at the specified path to figure out which tests your code has to pass.
// As with all other files in the assignment, the testing module for this file is stored in the API crate (this is the reason for the 'path' attribute in the code below)
// The reason I set it up like this is that it allows me to easily add additional tests when grading your projects, without changing any of your files, but you can still run my tests together with yours by specifying the right features (see below) :)
// directory.
//
// To run these tests, run the command `cargo test --features="X"` in the `solution` directory, with "X" a space-separated string of the features you are interested in testing.
//
// WARNING: DO NOT TOUCH THE BELOW CODE -- IT IS REQUIRED FOR TESTING -- YOU WILL LOSE POINTS IF I MANUALLY HAVE TO FIX YOUR TESTS
//The below configuration tag specifies the following things:
// 'cfg' ensures this module is only included in the source if all conditions are met
// 'all' is true iff ALL conditions in the tuple hold
// 'test' is only true when running 'cargo test', not 'cargo build'
// 'any' is true iff SOME condition in the tuple holds
// 'feature = X' ensures that the code is only compiled when the cargo command includes the flag '--features "<some-features>"' and some features includes X.
// I declared the necessary features in Cargo.toml
// (Hint: this hacking using features is not idiomatic behavior, but it allows you to run your own tests without getting errors on mine, for parts that have not been implemented yet)
// The reason for this setup is that you can opt-in to tests, rather than getting errors at compilation time if you have not implemented something.
// The "a" feature will run these tests specifically, and the "all" feature will run all tests.
#[cfg(all(test, any(feature = "a", feature = "all")))]
#[path = "../../api/fs-tests/a_test.rs"]
mod tests;
