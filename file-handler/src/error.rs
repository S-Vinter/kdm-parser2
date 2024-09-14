use std::{num::TryFromIntError, result::Result as std_result};

use glob::GlobError;
use rust_xlsxwriter::XlsxError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("IO failure")]
    IO(#[from] std::io::Error),

    #[error("int convertion failed")]
    TryFromInt(#[from] TryFromIntError),

    #[error("glob failure")]
    Glob(#[from] GlobError),

    #[error("sqlite operation failed")]
    Sqlite(#[from] rusqlite::Error),

    #[error("excel operation failed")]
    Xlsx(#[from] XlsxError),

    #[error("anyhow error")]
    Anyhow(#[from] anyhow::Error),

    #[error("xml operation failed")]
    QuickXML(#[from] quick_xml::Error),

    #[error("Current key doesn't exist")]
    NoSuchKey,
}

pub type Result<T> = std_result<T, Error>;
