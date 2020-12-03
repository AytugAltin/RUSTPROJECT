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
    AllocationError()
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
                write!(f,"No free room was found to allocate!")
        }
    }
}


