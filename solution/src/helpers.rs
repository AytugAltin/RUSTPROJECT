#![allow(dead_code)]

//Some more general testing utilities
use cplfs_api::controller::Device;
use cplfs_api::types::{Block, SuperBlock};
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


pub fn set_bitmapbit(sb: &SuperBlock, dev: &Device, mut i: u64, value:u64)-> Result<(), FileSystemError>{
    let mut blockindex =0;
    while sb.block_size <= i {
        blockindex += 1;
        i -= sb.block_size;
    }

    let bmblock_index = i + sb.bmapstart + blockindex;

    let mut block = dev.read_block(bmblock_index)?;

    let mut contents = block.contents_as_ref();
    //contents = contents & contents;
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

pub fn get_bytesarray_free_index(byte_array: &[u8])-> Result<u16, FileSystemError>{
    let mut byte_index = 0;

    for x in byte_array {
        if x.count_zeros() >0 {
            return Ok(byte_index);
        }
        byte_index += 1;
    }
    Err(FileSystemError::AllocationError())
}
