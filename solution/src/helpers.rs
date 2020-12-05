#![allow(dead_code)]

//Some more general testing utilities
use cplfs_api::controller::Device;
use cplfs_api::types::{Block, SuperBlock, Buffer};
use std::fs::{create_dir_all, remove_dir, remove_file, read};
use std::path::{Path, PathBuf};
use crate::a_block_support::FileSystem;
use cplfs_api::fs::BlockSupport;

use thiserror::Error;
use anyhow::Error;
use crate::filesystem_errors::FileSystemError;

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
        //let mut mutator = &[0b00000001u8];
        //block.write_data(& 0b00000000u8,0)
        dev.write_block(&block)?;
    }
    Ok(())
}

pub fn allocate_inoderegion(sb: &SuperBlock,dev: &Device){

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


pub fn get_nbitmapblocks(sb: &SuperBlock)-> u64{
    let mut nbitmapblocks = sb.ndatablocks / sb.block_size  ;

    if  sb.ndatablocks % sb.block_size  != 0{
        nbitmapblocks = nbitmapblocks +1;
        //if number is fraction add another block
    }
    return nbitmapblocks;
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
