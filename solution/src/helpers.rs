//! This file contains helper functions that are used to complete the assignment

use cplfs_api::controller::Device;
use cplfs_api::types::{
    Block, DirEntry, FType, Inode, InodeLike, SuperBlock, DINODE_SIZE, DIRENTRY_SIZE, DIRNAME_SIZE,
};

use crate::b_inode_support::FileSystem;
use cplfs_api::fs::{BlockSupport, InodeSupport};

use crate::b_inode_support::FSName;
use crate::filesystem_errors::FileSystemError;
use anyhow::Error;
use std::convert::TryInto;

// region PART_A
/// Writes a Superblock into the given device, error when something goes wrong
pub fn write_sb(sb: &SuperBlock, dev: &mut Device) -> Result<(), FileSystemError> {
    let mut firstblock = dev.read_block(0)?;
    firstblock.serialize_into(&sb, 0)?;
    dev.write_block(&firstblock)?;
    Ok(())
}

/// Alocates bitmapregion given a sevice and a superblock
pub fn allocate_bitmapregion(sb: &SuperBlock, dev: &mut Device) -> Result<(), FileSystemError> {
    let nbitmapblocks = get_nbitmapblocks(sb);
    let start = sb.bmapstart;
    let end = sb.bmapstart + nbitmapblocks;
    for i in start..end {
        let block = Block::new_zero(i, sb.block_size);
        dev.write_block(&block)?;
    }
    Ok(())
}

/// Calculates the number of bitmapblocks given a superblock
pub fn get_nbitmapblocks(sb: &SuperBlock) -> u64 {
    let mut nbitmapblocks = sb.ndatablocks / sb.block_size;

    if sb.ndatablocks % sb.block_size != 0 {
        nbitmapblocks = nbitmapblocks + 1;
        //if number is fraction add another block
    }
    return nbitmapblocks;
}

/// Calculates the number of inode blocks given a superblock
pub fn get_ninodeblocks(sb: &SuperBlock) -> u64 {
    let inodes_per_block = sb.block_size / *DINODE_SIZE;
    let mut ninodeblocks = sb.ninodes / inodes_per_block;
    //Calculate the amount of blocks needed for Inode
    if sb.ninodes % inodes_per_block != 0 {
        ninodeblocks = ninodeblocks + 1;
        //if number is fraction of a block ,then add another block
    }
    return ninodeblocks;
}

/// allocates the blocks for the Inode region given a superblock and a device
pub fn allocate_inoderegionblocks(
    sb: &SuperBlock,
    dev: &mut Device,
) -> Result<(), FileSystemError> {
    let ninodeblocks = sb.ninodes;
    let start = sb.inodestart;
    let end = sb.inodestart + ninodeblocks;
    for i in start..end {
        let block = Block::new_zero(i, sb.block_size);
        dev.write_block(&block)?;
    }
    Ok(())
}

/// Allocates the blocks for the Data region given a superblock and a device
pub fn allocate_dataregion(sb: &SuperBlock, dev: &mut Device) -> Result<(), FileSystemError> {
    let start = sb.datastart;
    let end = sb.datastart + sb.ndatablocks;
    for i in start..end {
        let block = Block::new_zero(i, sb.block_size);
        dev.write_block(&block)?;
    }
    Ok(())
}

/// Calculates a ceratin position by bits,bytes and blocks index in the dataregion given a superblock and a index
pub fn get_bit_byte_blockindex(
    sb: &SuperBlock,
    index: u64,
) -> Result<(u8, u16, u64), FileSystemError> {
    let mut blockindex = 0;
    let mut bitindex = index;
    while sb.block_size <= bitindex {
        blockindex += 1;
        bitindex -= sb.block_size;
    }

    let mut byteindex = 0;
    while 8 <= bitindex {
        byteindex += 1;
        bitindex -= 8;
    }
    let mut bitindexsmall = 0;
    while bitindex > 0 && bitindex < u8::max_value() as u64 {
        bitindexsmall += 1;
        bitindex -= 1;
    }
    // TO find the block where the bit must be changed

    let bmblock_index = bitindex + sb.bmapstart + blockindex;

    return Ok((bitindexsmall, byteindex, bmblock_index));
}

/// Sets the bitmap bit of a given filesystem that belongs to the datablock (data_index) to
/// 1 if n = true, 0 if n is false
pub fn set_bitmapbit(
    sb: &SuperBlock,
    dev: &mut Device,
    data_index: u64,
    n: bool,
) -> Result<(), FileSystemError> {
    check_data_index_outofbounds(sb, data_index)?;

    //let mut dev = fs.device.as_mut().ok_or_else(||FileSystemError::DeviceNotSet())?;

    let temp = get_bit_byte_blockindex(sb, data_index)?;
    let bitindex = temp.0;
    let byteindex = temp.1;
    let bmblock_index = temp.2;

    let mut bmblock = read_block(dev, bmblock_index)?;

    let to_write_block = set_bit_of_block(&mut bmblock, byteindex, bitindex, n)?;
    write_block(dev, &to_write_block)?;

    Ok(())
}

/// Quick check if a certain index of a data block is not out of bounds
fn check_data_index_outofbounds(sb: &SuperBlock, data_index: u64) -> Result<(), FileSystemError> {
    if data_index >= sb.ndatablocks {
        return Err(FileSystemError::IndexOutOfBounds());
    }
    Ok(())
}

/// Sets a certain bit of a block to 1 if n = true, 0 if n is false
fn set_bit_of_block(
    b: &mut Block,
    byteindex: u16,
    bitindex: u8,
    n: bool,
) -> Result<&mut Block, FileSystemError> {
    let byte_array = b.contents_as_ref();
    let byte = byte_array.get(usize::from(byteindex)).unwrap();

    if (byte >> bitindex).trailing_ones() == 0 && !n {
        return Err(FileSystemError::AllreadyFreeError());
    }

    if n {
        // to make bit 1
        let mut mutator = 0b00000001u8;
        mutator = mutator << bitindex; // FOR THE OR OPERATOR
        let to_write_byte = &[(*byte | mutator)]; // FOR THE OR OPERATOR
        b.write_data(to_write_byte, byteindex as u64)?;
    } else {
        // to make bit 0
        let mut mutator = 0b11111110u8;
        mutator = mutator.rotate_left(bitindex as u32);
        let to_write_byte = &[(*byte & mutator)]; // FOR THE AND OPERATOR
        b.write_data(to_write_byte, byteindex as u64)?;
    }

    return Ok(b);
}

///  Helper functions that reads the block at ith position of a given device
pub fn read_block(dev: &Device, i: u64) -> Result<Block, FileSystemError> {
    match dev.read_block(i) {
        Ok(block) => Ok(block),
        Err(e) => Err(FileSystemError::DeviceAPIError(e)),
    }
}

///  Helper functions that writes a block to the ith position of a given device
pub fn write_block(dev: &mut Device, b: &Block) -> Result<(), FileSystemError> {
    match dev.write_block(&b) {
        Ok(..) => Ok(()),
        Err(e) => Err(FileSystemError::DeviceAPIError(e)),
    }
}

/// Finds the index of the bitmap that is not set to 0
/// This index is where a new datablock is not in use and can be allocated
pub fn get_bytesarray_free_index(byte_array: &[u8]) -> Result<u16, FileSystemError> {
    let mut byte_index = 0;

    for x in byte_array {
        if x.count_zeros() > 0 {
            return Ok(byte_index);
        }
        byte_index += 1;
    }
    Err(FileSystemError::AllocationError())
}
/// Checks whether the superblock is valid or not and returns the result of the check
pub fn sb_valid(sb: &SuperBlock) -> bool {
    // Step 1: Check Order
    if !sb.inodestart == 1 {
        // Inode needs to start at index 1
        return false;
    }
    let ninodeblocks = get_ninodeblocks(sb);

    let nbitmapblocks = get_nbitmapblocks(sb);

    if !(sb.inodestart + ninodeblocks <= sb.bmapstart) {
        // check overlap between inodes region and bitmap region
        return false;
    }

    if !(sb.bmapstart + nbitmapblocks <= sb.datastart) {
        // check overlap between bitmap and data region
        return false;
    }

    if !(sb.datastart + sb.ndatablocks < sb.block_size) {
        // check overlap between bitmap and data region
        return false;
    }

    // Step 2: Check size
    if sb.nblocks < ninodeblocks + sb.ndatablocks + nbitmapblocks {
        return false;
    }
    return true;
}

//endregion

// region PART_B

/// Here we allocate the inodes of a given filesystem
pub fn allocate_inodes(fs: &mut FileSystem) -> Result<(), FileSystemError> {
    for i in 0..fs.superblock.ninodes {
        let i1 = <<FSName as InodeSupport>::Inode as InodeLike>::new(i, &FType::TFree, 0, 0, &[])
            .unwrap();
        //let mut inode = &Inode::new(i, i1);
        fs.i_put(&i1)?;
    }

    Ok(())
}

/// Securely gets the inode of the ith index
pub fn get_inode_block(
    fs: &FileSystem,
    i: u64,
    inodes_per_block: u64,
) -> Result<Block, FileSystemError> {
    let block;
    if i < fs.superblock.ninodes {
        let block_index = i / inodes_per_block;
        block = fs.b_get(fs.superblock.inodestart + block_index)?;
        Ok(block)
    } else {
        Err(FileSystemError::IndexOutOfBounds())
    }
}

/// Truncate the given inode of a Filesystem
pub fn trunc(fs: &mut FileSystem, ino: &mut Inode) -> Result<(), FileSystemError> {
    let size = get_inode_block_size(fs, ino);

    for j in 0..size as usize {
        let data_block = ino.disk_node.direct_blocks[j];
        if data_block >= fs.superblock.datastart
            && data_block < (fs.superblock.datastart + fs.superblock.ndatablocks)
        {
            let index = data_block - fs.superblock.datastart;
            fs.b_free(index)?;
            ino.disk_node.direct_blocks[j] = 0;
        }
    }

    ino.disk_node.size = 0;

    Ok(())
}

/// Calculates the number of inodes per block
pub fn get_inode_block_size(fs: &FileSystem, ino: &Inode) -> u64 {
    let mut size = ino.get_size() / fs.superblock.block_size;
    if ino.get_size() % fs.superblock.block_size != 0 {
        size = size + 1;
    }
    return size;
}

//endregion

//region PART_C

/// Converts a string to a char array that is used in a direntry
pub fn to_char_array(s: &str) -> Result<[char; DIRNAME_SIZE], Error> {
    let mut char_vec: Vec<char> = s.chars().collect();

    if char_vec.len() < DIRNAME_SIZE {
        char_vec.push('\0');
    }

    char_vec.resize(DIRNAME_SIZE, '\0');
    let slice = char_vec.as_slice();

    match slice.try_into() {
        Ok(result) => Ok(result),
        Err(e) => Err(Error::from(e)),
    }
}

/// Allocates the rootdirectory iniode in a given filesystem
pub fn allocate_rootdirectory(fs: &mut FileSystem) -> Result<(), FileSystemError> {
    let i1 =
        <<FSName as InodeSupport>::Inode as InodeLike>::new(1, &FType::TDir, 0, 0, &[]).unwrap();
    //let mut inode = &Inode::new(i, i1);
    fs.i_put(&i1)?;
    Ok(())
}

/// checks whether a string is a valid directoryname
pub fn is_valid_dirname(name: &str) -> bool {
    return name.replace(".", "0").chars().all(char::is_alphanumeric);
}

/// Get all directory entries, even if they are 0
pub fn get_direntries(
    fs: &FileSystem,
    inode: &Inode,
) -> Result<Vec<(DirEntry, u64)>, FileSystemError> {
    let mut list: Vec<(DirEntry, u64)> = vec![];

    let dirs_per_block = fs.superblock.block_size / *DIRENTRY_SIZE;
    let size = get_inode_block_size(fs, inode);

    // loop over entries
    for j in 0..size as usize {
        let data_block = inode.disk_node.direct_blocks[j];
        if data_block >= fs.superblock.datastart
            && data_block < (fs.superblock.datastart + fs.superblock.ndatablocks)
        {
            let block = fs.b_get(data_block)?;

            for i in 0..dirs_per_block {
                let block_dir_offset = i * *DIRENTRY_SIZE;
                let dir = block.deserialize_from::<DirEntry>(block_dir_offset)?;
                list.push((dir, block_dir_offset));
            }
        }
    }
    Ok(list)
}

/// writes a directory into a inode and allocates if there i not enough room
pub fn write_dir(
    fs: &mut FileSystem,
    inode: &mut Inode,
    dir: &DirEntry,
) -> Result<u64, FileSystemError> {
    let size = inode.get_size();
    let size_after = size + *DIRENTRY_SIZE; // This is the size after adding the direntry

    // The current inode has no room left
    // step 1 check if last data block has enough room
    let dirs_per_block = fs.superblock.block_size / *DIRENTRY_SIZE;
    let mut inode_blocks = 0;
    for i in &inode.disk_node.direct_blocks {
        if *i != 0 {
            inode_blocks += 1;
        }
    }
    let potential_dirs_room = inode_blocks * dirs_per_block;
    let dirs_needed = size_after / *DIRENTRY_SIZE;

    if potential_dirs_room < dirs_needed {
        // we need a new block
        let block_nr = fs.b_alloc()? + fs.superblock.datastart;
        add_block_to_inode(inode, block_nr)?;
        fs.i_put(&inode)?; // update new inode in fs
    }
    inode.disk_node.size = size_after;

    let size = get_inode_block_size(fs, inode);

    let dirs_per_block = fs.superblock.block_size / *DIRENTRY_SIZE;
    let mut offset = 0;
    for j in 0..size as usize {
        let data_block = inode.disk_node.direct_blocks[j];
        if data_block >= fs.superblock.datastart
            && data_block < (fs.superblock.datastart + fs.superblock.ndatablocks)
        {
            let mut block = fs.b_get(data_block)?;

            for i in 0..dirs_per_block {
                let block_dir_offset = i * *DIRENTRY_SIZE;
                let disk_dir = block.deserialize_from::<DirEntry>(block_dir_offset)?;
                if disk_dir.inum == 0 {
                    // we found an empty spot
                    block.serialize_into(&dir, block_dir_offset)?;

                    fs.b_put(&block)?;
                    offset += block_dir_offset;
                    return Ok(offset); // this is the inode offset of the new directories location
                }
            }
        }
        offset += fs.superblock.block_size;
    }
    return Err(FileSystemError::AllocationError());
}

/// adds a block to the inode
pub fn add_block_to_inode(inode: &mut Inode, block_nr: u64) -> Result<(), FileSystemError> {
    for i in 0..inode.disk_node.direct_blocks.len() {
        if inode.disk_node.direct_blocks[i] == 0 {
            inode.disk_node.direct_blocks[i] = block_nr;
            return Ok(());
        }
    }
    return Err(FileSystemError::AllocationError());
}

//endregion
