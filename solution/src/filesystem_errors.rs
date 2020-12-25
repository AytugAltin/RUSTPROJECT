//! This is a seperate file where the Errors for the filesystem have been defined

use thiserror::Error;

use cplfs_api::error_given::APIError;
use std::fmt;
use std::fmt::Formatter;

#[derive(Error, Debug)]
/// This structure defines the errors for the filestructure
/// All the potential errors are grouped here
pub enum FileSystemError {
    /// This error is raised whe the given superblock is invalid
    InvalidSuperBlock(),
    //TODO insert possible stuff
    /// This error is a conversion from an API error and is raised when there occurs when interacting with a Device
    DeviceAPIError(#[from] APIError),
    /// This error is raised whenever a Device is not set and we try to reach for the Devide
    /// Remember that the Device field in our filesystem is optional
    DeviceNotSet(),
    /// This error can be raised whenever allocating a block/inode/dir fails
    AllocationError(),
    /// This error will be raised when the to be freed block is allreads free
    AllreadyFreeError(),

    /// Error when de index is out of bound in a block
    IndexOutOfBounds(),
    /// Raised when the dir name is not valid
    InvalidDirname(),

    /// Raised when a problem occurs when free an inode
    INodeNotFreeable(),

    /// Raised when we are interacting with an inode that should be an Directory type but is not.
    INodeNotADirectory(),

    /// Raised whenever the inode on the disk is not up to date (not used)
    INodeNotFoundNotUpToDate(),

    /// When a directory has not been found when earching with its name.
    DirectoryNotFound(),
}

impl fmt::Display for FileSystemError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self{ //TODO add parameters in text
            FileSystemError::InvalidSuperBlock() =>
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
                write!(f,"Inode not up to date with the one in the filesystem"),
            FileSystemError::DirectoryNotFound() =>
                write!(f,"Directory not found in the filesystem")
        }
    }
}
