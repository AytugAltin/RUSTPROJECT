use thiserror::Error;
use cplfs_api::fs::{FileSysSupport, BlockSupport};
use std::error;
use std::fmt;
use cplfs_api::types::SuperBlock;
use cplfs_api::error_given::APIError;
use std::fmt::Formatter;
use std::cell::RefCell;

#[derive(Error,Debug)]
pub enum FileSystemError{
    InvalidSuperBlock(SuperBlock),
    //TODO insert possible stuff


    DeviceAPIError(#[from] APIError),
    DeviceNotSet(),
    AllocationError(),
    AllreadyFreeError(),
    IndexOutOfBounds(),
    InvalidDirname(),
    INodeNotFreeable(),
    INodeNotADirectory(),
    INodeNotFoundNotUpToDate()
}


impl fmt::Display for FileSystemError{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self{ //TODO add parameters in text
            FileSystemError::InvalidSuperBlock(SuperBlock) =>
                write!(f,"Invalid superblock"),
            FileSystemError::DeviceAPIError(api_error) =>
                write!(f,"Device API error"),
            FileSystemError::DeviceNotSet() =>
                write!(f,"FileSystem does not have a device set"),
            FileSystemError::AllocationError() =>
                write!(f,"No free room was found to allocate!"),
            FileSystemError::AllreadyFreeError() =>
                write!(f,"Cannot free the block at the requested index, it is already free"),
            FileSystemError::IndexOutOfBounds() =>
                write!(f,"Index Out of bounds!"),
            FileSystemError::INodeNotFreeable() =>
                write!(f,"Inode is not freeable, it still has links in the filesystem"),
            FileSystemError::InvalidDirname() =>
                write!(f,"The Provided directory name is not valid or too long or does contain illegal characters"),
            FileSystemError::INodeNotADirectory() =>
                write!(f,"Inode not a directory"),
            FileSystemError::INodeNotFoundNotUpToDate() =>
                write!(f,"Inode not up to date with the one in the filesystem")
        }
    }
}


