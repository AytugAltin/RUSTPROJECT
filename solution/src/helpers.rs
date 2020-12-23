#![allow(dead_code)]

//Some more general testing utilities
use cplfs_api::controller::Device;
use cplfs_api::types::{Block, SuperBlock, Buffer, DInode, DINODE_SIZE, Inode, FType, InodeLike,DIRENTRY_SIZE, DIRNAME_SIZE, DirEntry};
use std::fs::{create_dir_all, remove_dir, remove_file, read};
use std::path::{Path, PathBuf};
use crate::a_block_support::FileSystem;
use cplfs_api::fs::{BlockSupport, InodeSupport};

use thiserror::Error;
use anyhow::Error;
use crate::filesystem_errors::FileSystemError;
use cplfs_api::types::FType::TFree;
use crate::b_inode_support::FSName;
use std::borrow::Borrow;
use std::convert::TryInto;

// region PART_A

pub fn write_sb(sb: &SuperBlock, dev: &mut Device) -> Result<(), Error> {
    let mut firstblock = dev.read_block(0)?;
    firstblock.serialize_into(&sb, 0)?;
    dev.write_block(&firstblock)?;
    Ok(())
}

pub fn allocate_bitmapregion(sb: &SuperBlock, dev: &mut Device) -> Result<(), Error>{
    let nbitmapblocks = get_nbitmapblocks(sb);
    let start = sb.bmapstart;
    let end = sb.bmapstart + nbitmapblocks;
    for i in start..end{
        let block = Block::new_zero(i,sb.block_size);
        dev.write_block(&block)?;
    }
    Ok(())
}

pub fn get_nbitmapblocks(sb: &SuperBlock)-> u64{
    let mut nbitmapblocks = sb.ndatablocks / sb.block_size  ;

    if  sb.ndatablocks % sb.block_size  != 0{
        nbitmapblocks = nbitmapblocks +1;
        //if number is fraction add another block
    }
    return nbitmapblocks;
}

pub fn get_ninodeblocks(sb: &SuperBlock)-> u64{
    let inodes_per_block =  sb.block_size / *DINODE_SIZE;
    let mut ninodeblocks = sb.ninodes / inodes_per_block;
    //Calculate the amount of blocks needed for Inode
    if  sb.ninodes % inodes_per_block != 0{
        ninodeblocks = ninodeblocks +1;
        //if number is fraction of a block ,then add another block
    }
    return ninodeblocks;
}

pub fn allocate_inoderegionblocks(sb: &SuperBlock, dev: &mut Device) -> Result<(), Error>{
    let ninodeblocks = sb.ninodes;
    let start = sb.inodestart;
    let end = sb.inodestart + ninodeblocks;
    for i in start..end{
        let block = Block::new_zero(i,sb.block_size);
        dev.write_block(&block)?;
    }
    Ok(())
}

pub fn allocate_dataregion(sb: &SuperBlock, dev: &mut Device) -> Result<(), Error>{
    let start = sb.datastart;
    let end = sb.datastart + sb.ndatablocks;
    for i in start..end{
        let block = Block::new_zero(i,sb.block_size);
        dev.write_block(&block)?;
    }
    Ok(())
}


pub fn get_bit_byte_blockindex(sb: &SuperBlock, index: u64) -> Result<(u8,u16, u64), FileSystemError> {

    let mut blockindex = 0;
    let mut bitindex = index;
    while sb.block_size <= bitindex {
        blockindex += 1;
        bitindex -= sb.block_size;
    }

    let mut byteindex = 0;
    while 8<= bitindex {
        byteindex += 1;
        bitindex -= 8;
    }
    let mut bitindexsmall = 0;
    while bitindex > 0 && bitindex < u8::max_value() as u64 {
        bitindexsmall += 1;
        bitindex -=1;
    }
    // TO find the block where the bit must be changed

    let bmblock_index = bitindex + sb.bmapstart + blockindex;

    return Ok((bitindexsmall,byteindex,bmblock_index));
}

pub fn set_bitmapbit(fs: & mut FileSystem,  mut data_index: u64, n:bool)-> Result<(), FileSystemError>{

    check_data_index_outofbounds(&fs.superblock, data_index);

    let mut dev = fs.device.as_mut().ok_or_else(||FileSystemError::DeviceNotSet())?;

    let temp =  get_bit_byte_blockindex(&fs.superblock, data_index)?;
    let mut bitindex = temp.0;
    let mut byteindex = temp.1;
    let mut bmblock_index = temp.2;

    let mut bmblock = read_block(dev, bmblock_index)?;

    let to_write_block = set_bit_of_block(&mut bmblock, byteindex, bitindex,n)?;
    write_block(dev, &to_write_block)?;

    Ok(())

}

fn check_data_index_outofbounds(sb: &SuperBlock, mut data_index: u64)-> Result<(), FileSystemError>{
    if data_index >= sb.ndatablocks {
        return Err(FileSystemError::IndexOutOfBounds())
    }
    Ok(())
}

fn set_bit_of_block(b: &mut Block,byteindex:u16,bitindex:u8,n:bool)-> Result<&mut Block, FileSystemError>{
    let byte_array = b.contents_as_ref();
    let byte = byte_array.get(usize::from(byteindex)).unwrap();

    if (byte >> bitindex).trailing_ones() == 0 && !n{
        return Err(FileSystemError::AllreadyFreeError())
    }

    if n {
        // to make bit 1
        let mut mutator = 0b00000001u8;
        mutator = mutator << bitindex; // FOR THE OR OPERATOR
        let to_write_byte = &[(*byte | mutator)]; // FOR THE OR OPERATOR
        b.write_data(to_write_byte, byteindex as u64)?;
    }
    else{
        // to make bit 0
        let mut mutator = 0b11111110u8;
        mutator =mutator.rotate_left(bitindex as u32);
        let to_write_byte = &[(*byte & mutator)]; // FOR THE AND OPERATOR
        b.write_data(to_write_byte, byteindex as u64)?;
    }

    return Ok(b);
}

pub fn read_block(dev: &Device, i: u64) -> Result<Block, FileSystemError> {
    match dev.read_block(i) {
        Ok(mut block) => Ok(block),
        Err(e) => Err(FileSystemError::DeviceAPIError(e))
    }
}

pub fn write_block(dev: &mut Device, b: &Block) -> Result<(), FileSystemError> {
    match dev.write_block(&b) {
        Ok(..) => Ok(()),
        Err(e) => Err(FileSystemError::DeviceAPIError(e))
    }
}


pub fn get_bytesarray_free_index(byte_array: &[u8])-> Result<u16, FileSystemError>{
    let mut byte_index = 0;

    for x in byte_array{
        if x.count_zeros() >0 {
            return Ok(byte_index);
        }
        byte_index += 1;
    }
    Err(FileSystemError::AllocationError())
}

//endregion


// region PART_B

pub fn allocate_inodes(fs: & mut FileSystem) -> Result<(), FileSystemError> {


    for i in 0..fs.superblock.ninodes {
        let i1 = <<FSName as InodeSupport>::Inode as InodeLike>::new(
            i,
            &FType::TFree,
            0,
            0,
            &[],
        ).unwrap();
        //let mut inode = &Inode::new(i, i1);
        fs.i_put(&i1)?;
    }

    allocate_rootdirectory(fs);
    Ok(())
}


pub fn get_inode_block(fs: & FileSystem,i: u64,inodes_per_block:u64) -> Result<Block, FileSystemError>
{
    let mut block;
    if i < fs.superblock.ninodes {
        let block_index = i / inodes_per_block;
        block = fs.b_get(fs.superblock.inodestart + block_index)?;
        Ok(block)
    }
    else{
        Err(FileSystemError::IndexOutOfBounds())
    }
}



pub fn trunc(fs: &mut FileSystem, ino: &mut Inode ) -> Result<(), FileSystemError> {

    let mut size = get_inode_block_size(fs,ino);

    for j in 0..size as usize {
        let data_block = ino.disk_node.direct_blocks[j];
        if data_block >= fs.superblock.datastart && data_block < (fs.superblock.datastart + fs.superblock.ndatablocks) {
            let index = data_block - fs.superblock.datastart;
            fs.b_free(index)?;
            ino.disk_node.direct_blocks[j] = 0;
        }
    }

    ino.disk_node.size = 0;

    Ok(())
}

pub fn get_inode_block_size(fs:  &FileSystem, ino:  &Inode ) -> u64 {
    let mut size = ino.get_size() / fs.superblock.block_size;
    if ino.get_size() % fs.superblock.block_size != 0 {
        size = size + 1;
    }
    return size;


}




//endregion



//region PART_C

pub fn to_char_array(s: &str) -> Result< [char;DIRNAME_SIZE], Error> {
    let mut char_vec: Vec<char> = s.chars().collect();

    if char_vec.len() < DIRNAME_SIZE {
        char_vec.push('\0');
    }

    char_vec.resize(DIRNAME_SIZE,'\0');
    let slice = char_vec.as_slice();


    match slice.try_into() {
        Ok(result) => Ok(result),
        Err(e) => Err(Error::from(e))
    }
}


pub fn allocate_rootdirectory(fs: & mut FileSystem) -> Result<(), FileSystemError> {
    let i1 = <<FSName as InodeSupport>::Inode as InodeLike>::new(
        1,
        &FType::TDir,
        0,
        0,
        &[],
    ).unwrap();
    //let mut inode = &Inode::new(i, i1);
    fs.i_put(&i1)?;
    Ok(())
}


pub fn is_valid_dirname(name: &str)-> bool{
    return  name.replace(".", "0").chars().all(char::is_alphanumeric)
}

/**
Get directory enties
**/
pub fn get_direntries(fs: & FileSystem, inode: & Inode) -> Result<Vec<(DirEntry, u64)>, FileSystemError> {

    let mut list : Vec<(DirEntry, u64)> = vec![];

    let dirs_per_block = fs.superblock.block_size / *DIRENTRY_SIZE;
    let mut size = get_inode_block_size(fs ,inode);

    // loop over entries
    for j in 0..size as usize{
        let data_block = inode.disk_node.direct_blocks[j];
        if data_block >= fs.superblock.datastart && data_block < (fs.superblock.datastart + fs.superblock.ndatablocks) {
            let index = data_block - fs.superblock.datastart;
            let block = fs.b_get(index)?;

            for index in 0..dirs_per_block{
                let block_dir_offset = index * *DIRENTRY_SIZE;
                let dir = block.deserialize_from::<DirEntry>(block_dir_offset)?;
                list.push((dir,block_dir_offset));
            }
        }
    }
    Ok(list)
}

pub fn write_dir(fs: & FileSystem, inode: & Inode, dir: &DirEntry){
    let nlink =inode.get_nlink();
    let min_size_after_link = (nlink +1) * *DIRENTRY_SIZE ; // This is the size after adding the direntry
    if min_size_after_link > inode.get_size(){
        // needs more room
        //step 1 check if last data block has enough room
        let dirs_per_block = fs.superblock.block_size / *DIRENTRY_SIZE;
        //if inode.disk_node.direct_blocks.len() *


    }
}

pub fn compare_inodes(inodeA: &Inode, inodeb: &Inode) -> bool{
    if (inodeA.disk_node == inodeb.disk_node) {
        return true
    }
    return false
}










//endregion