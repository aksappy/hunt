use bincode;
use lang::Language;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs::{read, File};
use std::io::{self, Error, Read, Write};
use std::path::Path;
use stop_words;
use strsim::levenshtein;
use unicode_segmentation::UnicodeSegmentation;
use walkdir::{DirEntry, WalkDir};
mod lang;

#[derive(Serialize, Deserialize, Debug)]
struct Item {
    filename: String, // To track the source file
    bwt: String,
    suffix_array: Vec<usize>,
    occ_table: HashMap<char, Vec<usize>>,
    tokens: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Index {
    items: Vec<Item>,
}

pub fn read_by_path(path: &Path) -> io::Result<String> {
    let mut file = File::open(path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    Ok(contents)
}

pub fn read_by_path_string(path_string: &str) -> io::Result<String> {
    read_by_path(Path::new(path_string))
}

#[cfg(test)]
mod tests {
    use core::panic;
    use std::{env::current_dir, os};

    use crate::{read_by_path, read_by_path_string};

    #[test]
    fn should_return_file_content_when_file_exists() {
        let result = read_by_path_string("./Cargo.toml");
        match result {
            Ok(_) => println!("Success"),
            Err(_) => panic!("failed"),
        }
    }

    #[test]
    fn should_return_error_when_file_does_not_exist() {
        let result = read_by_path_string("./Cargo1.toml");
        match result {
            Ok(_) => panic!("failed"),
            Err(_) => println!("Success"),
        }
    }
}
