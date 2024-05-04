use regex::Regex;
use std::{
    collections::{HashMap, HashSet},
    fs::File,
    io::{self, BufRead},
    path::Path,
};
use unidecode::unidecode;

pub fn get_word_count(filepath: &Path) -> HashMap<String, u8> {
    let mut word_count: HashMap<String, u8> = HashMap::new();
    get_lines(filepath).for_each(|f| match f {
        Ok(v) => count_words(v, &mut word_count),
        Err(err) => panic!("get_word_count {:?}", err),
    });
    word_count
}

// Returns an Iterator to the Reader of the lines of the file.
pub fn get_lines(filepath: &Path) -> io::Lines<io::BufReader<File>> {
    match File::open(filepath) {
        Ok(file) => io::BufReader::new(file).lines(),
        Err(err) => panic!("Function:get_lines Error:{:?}", err),
    }
}

fn count_words(line: String, word_count: &mut HashMap<String, u8>) {
    for word in line.split_whitespace() {
        let token = remove_punctuation(word);
        if !is_stopword(word) {
            let entry = word_count.entry(token).or_insert(0);
            *entry += 1;
        };
    }
}

pub fn is_stopword(word: &str) -> bool {
    let stop_words = HashSet::from(["los", "las", "la", "de", "y", "este", "esta"]);
    stop_words.contains(word)
}

pub fn remove_punctuation(input: &str) -> String {
    let re = Regex::new(r"[[:punct:]]").unwrap();
    let word = re.replace_all(input, "");
    unidecode(&word).to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remove_punctuation() {
        let text = "I'm, gonna be... formated!";
        assert_eq!(remove_punctuation(text), "im gonna be formated");
    }
}
