//! Thin wrapper around lindera + ko-dic that turns tokens into morpheme lists.

use std::borrow::Cow;
use std::path::Path;

use lindera::LinderaResult;
use lindera::dictionary::{load_dictionary, load_user_dictionary};
use lindera::mode::Mode;
use lindera::segmenter::Segmenter;

#[derive(Clone, Debug)]
pub struct Morph {
    pub form: String,
    pub pos: String,
}

#[derive(Clone, Debug)]
pub struct Tok {
    pub surface: String,
    pub start: usize,
    pub end: usize,
    pub morphs: Vec<Morph>,
}

impl Tok {
    pub fn has_pos(&self, pos: &str) -> bool {
        self.morphs.iter().any(|m| m.pos == pos)
    }
}

pub struct Analyzer {
    segmenter: Segmenter,
}

impl Analyzer {
    /// Loads the embedded ko-dic. `user_dict_csv` holds setting-card names as
    /// `surface,NNP,reading` lines so names are not split into pieces.
    pub fn new(user_dict_csv: Option<&Path>) -> LinderaResult<Self> {
        let dictionary = load_dictionary("embedded://ko-dic")?;
        let user = match user_dict_csv.and_then(|p| p.to_str()) {
            Some(path) => Some(load_user_dictionary(path, &dictionary.metadata)?),
            None => None,
        };
        Ok(Self { segmenter: Segmenter::new(Mode::Normal, dictionary, user) })
    }

    /// Analyzes `text` and shifts offsets by `base` so they point into the whole document.
    pub fn analyze(&self, text: &str, base: usize) -> LinderaResult<Vec<Tok>> {
        let tokens = self.segmenter.segment(Cow::Borrowed(text))?;
        let mut out = Vec::with_capacity(tokens.len());
        for mut t in tokens {
            let surface = t.surface.to_string();
            let (start, end) = (t.byte_start + base, t.byte_end + base);
            let details = t.details();
            let pos = details.first().copied().unwrap_or("UNK").to_string();
            let expression = details.get(7).copied().unwrap_or("*").to_string();
            let morphs = parse_morphs(&surface, &pos, &expression);
            out.push(Tok { surface, start, end, morphs });
        }
        Ok(out)
    }
}

/// ko-dic stores inflected forms like 보았 as `보/VV/*+았/EP/*` in the expression field.
fn parse_morphs(surface: &str, pos: &str, expression: &str) -> Vec<Morph> {
    if expression != "*" && expression.contains('/') {
        let morphs: Vec<Morph> = expression
            .split('+')
            .filter_map(|part| {
                let mut it = part.split('/');
                let form = it.next()?;
                let pos = it.next()?;
                Some(Morph { form: form.to_string(), pos: pos.to_string() })
            })
            .collect();
        if !morphs.is_empty() {
            return morphs;
        }
    }
    if pos.contains('+') {
        return pos
            .split('+')
            .enumerate()
            .map(|(i, p)| Morph { form: if i == 0 { surface.to_string() } else { String::new() }, pos: p.to_string() })
            .collect();
    }
    vec![Morph { form: surface.to_string(), pos: pos.to_string() }]
}
