use crate::automata::dfa::Dfa;
use crate::error::LexError;
use crate::lex_file::parser::LexFile;

/// Trait for code emitters. Each target language implements this.
pub trait CodeEmitter {
    /// Emit the complete output file content.
    fn emit(&self, lex_file: &LexFile, dfa: &Dfa) -> Result<String, LexError>;
}
