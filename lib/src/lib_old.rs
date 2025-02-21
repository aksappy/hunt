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

struct HuntError {
    pub message: String,
}

pub struct Hunt {
    pub index_file: String,
    pub language: Language,
}

#[derive(Serialize, Deserialize, Debug)]
struct FMIndex {
    filename: String, // To track the source file
    bwt: String,
    suffix_array: Vec<usize>,
    occ_table: HashMap<char, Vec<usize>>,
    tokens: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FMIndexCollection {
    indexes: Vec<FMIndex>,
}

impl Hunt {
    pub fn new_with_english(index_file: String) -> Self {
        Self {
            index_file,
            language: Language::ENGLISH,
        }
    }
    pub fn new(index_file: String, language: Language) -> Self {
        Self {
            index_file,
            language,
        }
    }

    pub fn read_file_contents(path: &Path) -> io::Result<String> {
        let mut file = File::open(path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        Ok(contents)
    }

    pub fn index_directory(&self, directory: String) -> Result<(), Error> {
        let mut contents = Vec::new();

        for entry in WalkDir::new(directory).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if matches!(ext, "txt" | "java" | "csv") {
                        match Self::read_file_contents(path) {
                            Ok(data) => {
                                self.index_file()
                            },
                            Err(err) => eprintln!("Warning: Could not read {:?}: {}", path, err),
                        }
                    }
                }
            }
        }

        let walker = WalkDir::new(directory).into_iter();
        let paths: Vec<FMIndex> = walker
            .filter_entry(|e| !is_hidden(e))
            .filter(|x| is_valid_dir_entry(x))
            .map(|x| self.index_file(x.path().to_str().unwrap()).unwrap())
            .collect();
        let collection = FMIndexCollection { indexes: paths };
        return self.save_fm_indexes(&collection, "index.bin");
    }

    //TODO index a single file
    //TODO index multiple files
    //TODO index a directory of files
    //TODO index text

    //TODO save bin data to user input filename
    //TODO save bin data to user input filename and directory
    //TODO save bin data to default directory

    /// Tokenizes text into unique lowercase words
    /// This method also filters out stop words
    pub fn tokenize(&self, text: &str) -> Vec<String> {
        let mut tokens: HashSet<String> = HashSet::new();
        let words = stop_words::get(self.language.to());
        for word in text.unicode_words() {
            if !words.contains(&word.to_string()) {
                tokens.insert(word.to_lowercase());
            }
        }
        tokens.into_iter().collect()
    }

    fn suffix_array(text: &str) -> Vec<usize> {
        let mut suffixes: Vec<_> = (0..text.len()).collect();
        suffixes.sort_by_key(|&i| &text[i..]);
        suffixes
    }

    /// Builds the Burrows-Wheeler Transform (BWT) from a given text
    pub fn build_bwt(text: &str) -> String {
        let mut rotations: Vec<String> = (0..text.len())
            .map(|i| format!("{}{}", &text[i..], &text[..i]))
            .collect();
        rotations.sort();
        rotations
            .iter()
            .map(|s| s.chars().last().unwrap())
            .collect()
    }

    /// Counts the occurrences of each character in the BWT
    fn count_occurrences(&self, bwt: &str, target: char) -> Vec<usize> {
        let mut counts = vec![0; bwt.len() + 1];
        for (i, c) in bwt.chars().enumerate() {
            counts[i + 1] = counts[i] + (if c == target { 1 } else { 0 });
        }
        counts
    }

    /// Searches for a query in the BWT using the FM-Index
    fn search_fm_index(&self, bwt: &str, sa: &[usize], query: &str) -> Option<usize> {
        let mut left = 0;
        let mut right = sa.len();

        for c in query.chars().rev() {
            let occ = self.count_occurrences(bwt, c);
            left = occ[left];
            right = occ[right];

            if left >= right {
                return None;
            }
        }

        Some(sa[left])
    }

    /// Saves multiple FM-Indexes into a single file
    pub fn save_fm_indexes(&self, indexes: &FMIndexCollection, path: &str) -> std::io::Result<()> {
        let encoded = bincode::serialize(indexes).unwrap();
        let mut file = File::create(path)?;
        file.write_all(&encoded)?;
        Ok(())
    }

    /// Loads multiple FM-Indexes from a file
    pub fn load_fm_indexes(&self, path: &str) -> std::io::Result<FMIndexCollection> {
        let data = read(path)?;
        let indexes: FMIndexCollection = bincode::deserialize(&data).unwrap();
        Ok(indexes)
    }

    pub fn compute_occurrences(&self, bwt: &str) -> HashMap<char, Vec<usize>> {
        let mut occ_table: HashMap<char, Vec<usize>> = HashMap::new();

        for (i, c) in bwt.chars().enumerate() {
            let counts = occ_table.entry(c).or_insert(vec![0; bwt.len() + 1]);
            counts[i + 1] = counts[i] + 1;
        }

        occ_table
    }

    /// Search for an exact match across all FM-Indexes
    pub fn search_exact(&self, query: &str, indices: &[FMIndex]) -> Vec<(String, usize)> {
        let mut results = Vec::new();

        for index in indices {
            if index.tokens.contains(&query.to_string()) {
                results.push((index.filename.clone(), index.suffix_array[0]));
            }
        }

        results
    }

    /// Search for fuzzy matches across all FM-Indexes
    pub fn search_fuzzy(
        &self,
        query: &str,
        max_distance: usize,
        indices: &[FMIndex],
    ) -> Vec<(String, String, usize)> {
        let mut results = Vec::new();

        for index in indices {
            for token in &index.tokens {
                let distance = levenshtein(query, token);
                if distance <= max_distance {
                    results.push((index.filename.clone(), token.clone(), distance));
                }
            }
        }

        // Sort by smallest distance first
        results.sort_by_key(|k| k.2);
        results
    }

    /// Indexes a file and returns an FMIndex
    pub fn index_file(&self, file_path: &str) -> std::io::Result<FMIndex> {
        let content = std::fs::read_to_string(file_path)?;
        let tokens = self.tokenize(&content);
        let bwt = Self::build_bwt(&content);
        let sa = Self::suffix_array(&content);
        let occ_table = self.compute_occurrences(&bwt);

        Ok(FMIndex {
            filename: file_path.to_string(),
            bwt,
            suffix_array: sa,
            occ_table,
            tokens,
        })
    }

    /// Indexes multiple files and returns an FMIndexCollection
    pub fn index_files(&self, file_paths: &[&str]) -> std::io::Result<FMIndexCollection> {
        let mut indexes = Vec::new();
        for path in file_paths {
            let index = self.index_file(path)?;
            indexes.push(index);
        }
        Ok(FMIndexCollection { indexes })
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    #[test]
    fn hunt_initialization_should_return_english_as_default_language() {
        let hunt = Hunt::new_with_english("test.txt".to_string());
        assert_eq!(hunt.language, Language::ENGLISH);
    }

    #[test]
    fn hunt_index_directory_must_generate_index_file() {
        let hunt = Hunt::new_with_english("index.bin".to_string());
        hunt.index_directory("./examples".to_string());
        let path = Path::new("ID");
        assert!(Path::new("ID").exists() == true);
        std::fs::remove_file(path);
    }
    // error scenario when file cannot be created for some reason

    #[test]
    fn hunt_index_directory_must_generate_index_from_walking_directory() {
        let hunt = Hunt::new_with_english("index.bin".to_string());
        hunt.index_directory("./examples".to_string());
        let indices = hunt.load_fm_indexes("index.bin");
        match indices {
            Ok(indices) => {
                let response = hunt.search_exact("DIR2", &indices.indexes);
                assert!(response.len() > 0);
            }
            Err(_) => assert!(false),
        }
    }

    #[test]
    fn hunt_index_directory_should_only_select_text_files() {
        assert!(false);
    }
    #[test]
    fn hunt_index_directory_should_allow_skipping_hidden_folders() {
        assert!(false);
    }
    #[test]
    fn hunt_index_directory_should_allow_skipping_provided_folders() {
        assert!(false);
    }
}

fn is_hidden(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| s.starts_with("."))
        .unwrap_or(false)
}
