use thiserror::Error;
use cplfs_api::fs::{FileSysSupport, BlockSupport};
use std::error;
use std::fmt;
use cplfs_api::types::SuperBlock;
use cplfs_api::error_given::APIError;
use std::fmt::Formatter;

#[derive(Error,Debug)]
pub enum FileSystemError{
    InvalidSuperBlock(SuperBlock),
    //TODO insert possible stuff

    PathError(#[from] APIError),// TODO maybe needs variable path
}


impl fmt::Display for FileSystemError{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self{ //TODO add parameters in text
            FileSystemError::InvalidSuperBlock(SuperBlock) =>
                write!(f,"Invalid superblock"),
            /*FileSystemError::PathAlreadyExists(path) =>
                write!(f,"Invalid path or allready in use")*/ //todo remove or use
            FileSystemError::PathError(api_error) =>
                write!(f,"Invalid path or already in use")
        }
    }
}
