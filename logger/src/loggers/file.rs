use std::{fs::File, io::Write, path::PathBuf};

use crate::Logger;

pub enum FileConflictBehavior {
    AppendNumber,
    Append,
    Error,
    Overwrite,
    RenameOld,
}

pub struct FileLogger {
    file: File,
}

impl Logger for FileLogger {
    fn log(&mut self, message: &crate::LogMessage) -> bool {
        microseh::try_seh(|| {
            let content = format!("({}) | {:#?} : {}\n", message.time.format("%Y-%b-%d %I:%M%p"), message.severity, message.content);
            self.file.write_all(content.as_bytes()).is_ok()
        }).is_ok()
    }
}

impl FileLogger {
    pub fn new(file: PathBuf, behavior: FileConflictBehavior) -> Result<Self, std::io::Error> {
        let exists = file.try_exists()?;
        let file = if exists {
            match behavior {
                FileConflictBehavior::AppendNumber => {
                    let mut new_file = file.clone();
                    let mut counter = 1;
                    while new_file.try_exists()? {
                        new_file.set_file_name(format!(
                            "{}_{}",
                            file.file_stem().unwrap().to_string_lossy(),
                            counter
                        ));
                        if let Some(extension) = file.extension() {
                            new_file.set_extension(extension);
                        }
                        counter += 1;
                    }
                    File::create(new_file)?
                }
                FileConflictBehavior::Append => {
                    File::options().append(true).open(file)?
                }
                FileConflictBehavior::Error => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::AlreadyExists,
                        "File already exists",
                    ));
                }
                FileConflictBehavior::Overwrite => {
                    File::create(file)?
                }
                FileConflictBehavior::RenameOld => {
                    let mut old_file = file.clone();
                    let mut counter = 1;
                    while old_file.try_exists()? {
                        old_file.set_file_name(format!(
                            "{}_old_{}",
                            file.file_stem().unwrap().to_string_lossy(),
                            counter
                        ));
                        if let Some(extension) = file.extension() {
                            old_file.set_extension(extension);
                        }
                        counter += 1;
                    }
                    std::fs::rename(&file, old_file)?;
                    File::create(file)?
                }
            }
        } else {
            File::create(file)?
        };
        Ok(Self {
            file,
        })
    }
}