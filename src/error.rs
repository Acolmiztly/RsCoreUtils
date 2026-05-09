use thiserror::Error;
use crate::commands;

#[derive(Error, Debug)]
pub enum CliError {
    #[error(transparent)]
    Cat(#[from] commands::cat::CatError),
    #[error(transparent)]
    Grep(#[from] commands::grep::GrepError),
    // Add new tools here: #[error(transparent)] Ls(#[from] commands::ls::LsError),
}