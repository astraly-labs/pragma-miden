use miden_client::Word;
/// Word to MASM
pub fn word_to_masm(word: Word) -> String {
    word.iter()
        .map(|x| x.as_int().to_string())
        .collect::<Vec<_>>()
        .join(".")
}
