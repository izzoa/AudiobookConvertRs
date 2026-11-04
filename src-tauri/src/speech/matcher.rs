//! Finding a chapter's title in what the narrator said.
//!
//! Catalog titles ("Chapter 4: Outlaws", "Part II: Right Here, Right Now")
//! and transcripts ("Chapter four. Outlaws.") differ in case, punctuation and
//! how numbers are written, so both are normalized first. A title counts as
//! spoken when it is announced: "chapter" or "part" with the right number at
//! the start of a sentence, or most of its words close together. A passing
//! mention ("as we saw in chapter four") is not an announcement.

use crate::speech::Word;

/// How long a spoken title may take, from its first matched word to its last
const SPAN_MS: u64 = 8_000;
/// Share of a title's words that must be heard, in order
const WORD_SHARE: f64 = 0.6;
/// A silence this long before a word starts a new sentence
const SENTENCE_GAP_MS: u64 = 300;
/// How far apart the title's number or division word may be from the words
/// that follow it and still belong to the same announcement
const LEAD_IN_GAP_MS: u64 = 2_000;

/// Words that name a numbered division of a book
const DIVISIONS: &[&str] = &["chapter", "part", "book", "section"];

/// Common words that say little about which title was heard
const STOP_WORDS: &[&str] = &[
    "the", "a", "an", "of", "and", "to", "in", "on", "for", "with", "at", "by", "from",
];

/// One normalized word, with the times of the transcript word(s) it came from
#[derive(Debug, Clone, PartialEq)]
struct Token {
    text: String,
    start_ms: u64,
    /// The word before ended a sentence, or a silence came before it
    starts_sentence: bool,
}

/// A spoken title
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HeardTitle {
    /// Where it starts, in the transcript's time
    pub start_ms: u64,
    /// It was matched on its words, after a "chapter 4" Whisper did not
    /// recognize ("Number 5. China…"), so the break may come earlier
    pub lead_in_unheard: bool,
}

/// Where a spoken title starts, in the transcript's time, when it is heard.
#[cfg(test)]
fn find_title(title: &str, words: &[Word], estimate_ms: u64) -> Option<u64> {
    find_announcement(title, words, estimate_ms).map(|h| h.start_ms)
}

/// The spoken title, when it is heard. Among several, the most
/// announcement-like wins, then the one nearest `estimate_ms`.
pub fn find_announcement(title: &str, words: &[Word], estimate_ms: u64) -> Option<HeardTitle> {
    let wanted: Vec<String> = normalize(title).into_iter().map(|t| t.text).collect();
    if wanted.is_empty() || is_credits(&wanted) {
        return None;
    }
    let heard = tokenize(words);
    let division = wanted
        .windows(2)
        .find(|w| DIVISIONS.contains(&w[0].as_str()) && w[1].starts_with('#'))
        .map(|w| (w[0].clone(), w[1].clone()));
    // The words that describe the chapter, without "chapter 4" or filler
    let descriptive: Vec<&String> = {
        let words: Vec<&String> = wanted
            .iter()
            .filter(|w| {
                !STOP_WORDS.contains(&w.as_str())
                    && !DIVISIONS.contains(&w.as_str())
                    && !w.starts_with('#')
            })
            .collect();
        if words.is_empty() {
            wanted.iter().collect()
        } else {
            words
        }
    };
    let needed = ((descriptive.len() as f64 * WORD_SHARE).ceil() as usize)
        .max(descriptive.len().min(2))
        .max(1);
    // The title's number and division word, which may be heard just before
    // its descriptive words
    let lead_in: Vec<&String> = wanted
        .iter()
        .filter(|w| DIVISIONS.contains(&w.as_str()) || w.starts_with('#'))
        .collect();

    // (rank, distance to the estimate, title) for every candidate
    let mut best: Option<(u8, u64, HeardTitle)> = None;
    for (i, token) in heard.iter().enumerate() {
        let mut rank = 0;
        if let Some((name, number)) = &division {
            let announced =
                &token.text == name && heard.get(i + 1).is_some_and(|next| &next.text == number);
            if announced {
                // Title words after the number make it unmistakable
                let words_follow = heard[i + 2..]
                    .iter()
                    .take_while(|t| t.start_ms <= token.start_ms + SPAN_MS)
                    .any(|t| descriptive.iter().any(|w| **w == t.text));
                if words_follow {
                    rank = 3;
                } else if token.starts_sentence {
                    rank = 2;
                }
            }
        }
        let mut start = token.start_ms;
        let mut lead_in_unheard = false;
        if rank == 0 && descriptive.iter().any(|w| **w == token.text) {
            let matched = in_order(&heard[i..], &descriptive, token.start_ms + SPAN_MS);
            if matched >= needed && (token.starts_sentence || matched == descriptive.len()) {
                rank = 1;
                start = announced_from(&heard, i, &lead_in);
                lead_in_unheard = !lead_in.is_empty() && start == token.start_ms;
            }
        }
        if rank == 0 {
            continue;
        }
        let distance = start.abs_diff(estimate_ms);
        let better = match best {
            None => true,
            Some((r, d, _)) => rank > r || (rank == r && distance < d),
        };
        if better {
            let found = HeardTitle {
                start_ms: start,
                lead_in_unheard,
            };
            best = Some((rank, distance, found));
        }
    }
    best.map(|(_, _, found)| found)
}

/// Where a title matched by its words at `heard[i]` starts: at the title's
/// number or division word spoken just before, when Whisper heard only part
/// of the announcement ("10. Tomorrow's World" for "Chapter 10. Tomorrow's
/// World"), so the chapter does not start after them.
fn announced_from(heard: &[Token], i: usize, lead_in: &[&String]) -> u64 {
    let mut first = i;
    while first > 0 {
        let before = &heard[first - 1];
        let close = heard[first].start_ms.saturating_sub(before.start_ms) <= LEAD_IN_GAP_MS;
        if !close || !lead_in.iter().any(|w| **w == before.text) {
            break;
        }
        first -= 1;
    }
    // A number that ends the sentence before is not part of this announcement
    if heard[first].starts_sentence {
        heard[first].start_ms
    } else {
        heard[i].start_ms
    }
}

/// How many of `wanted` appear in order in `heard`, from its first token,
/// before `until_ms`. Title words may be skipped, but never revisited.
fn in_order(heard: &[Token], wanted: &[&String], until_ms: u64) -> usize {
    let mut position = 0;
    let mut count = 0;
    for token in heard.iter().take_while(|t| t.start_ms <= until_ms) {
        if let Some(found) = wanted[position..].iter().position(|w| **w == token.text) {
            position += found + 1;
            count += 1;
            if position == wanted.len() {
                break;
            }
        }
    }
    count
}

fn is_credits(words: &[String]) -> bool {
    words.iter().any(|w| w == "credits")
        && words
            .iter()
            .all(|w| matches!(w.as_str(), "opening" | "end" | "closing" | "credits"))
}

/// The transcript as normalized tokens
fn tokenize(words: &[Word]) -> Vec<Token> {
    let mut tokens = Vec::new();
    // The last word that said something; marks like a lone `"` do not count,
    // but Whisper puts them at breaks, so they start a sentence
    let mut previous: Option<&Word> = None;
    let mut marks_since = false;
    for word in words {
        let normalized = normalize(&word.text);
        if normalized.is_empty() {
            marks_since = true;
            continue;
        }
        let after_break = marks_since
            || previous.is_none_or(|p| {
                p.text
                    .trim_end()
                    .ends_with(['.', '!', '?', ',', ';', ':', '"'])
                    || word.start_ms.saturating_sub(p.end_ms) >= SENTENCE_GAP_MS
            });
        for (k, mut token) in normalized.into_iter().enumerate() {
            token.start_ms = word.start_ms;
            token.starts_sentence = k == 0 && after_break;
            tokens.push(token);
        }
        previous = Some(word);
        marks_since = false;
    }
    join_numbers(tokens)
}

/// Lower-cased words with punctuation dropped and numbers written `#N`
fn normalize(text: &str) -> Vec<Token> {
    let cleaned: String = text
        .chars()
        .map(|c| match c {
            '\'' | '\u{2019}' => '\0',
            c if c.is_alphanumeric() => c,
            _ => ' ',
        })
        .filter(|c| *c != '\0')
        .collect();
    let raw: Vec<String> = cleaned
        .split_whitespace()
        .map(|w| w.to_lowercase())
        .collect();
    let mut tokens = Vec::with_capacity(raw.len());
    for (i, word) in raw.iter().enumerate() {
        let after_division = i > 0 && DIVISIONS.contains(&raw[i - 1].as_str());
        let alone = raw.len() == 1;
        let text = number(word, after_division || alone)
            .map(|n| format!("#{}", n))
            .unwrap_or_else(|| word.clone());
        tokens.push(Token {
            text,
            start_ms: 0,
            starts_sentence: false,
        });
    }
    join_numbers(tokens)
}

/// "twenty" "one" becomes one number, as "twenty-one" does
fn join_numbers(tokens: Vec<Token>) -> Vec<Token> {
    let mut joined: Vec<Token> = Vec::with_capacity(tokens.len());
    for token in tokens {
        if let (Some(last), Some(unit)) = (joined.last_mut(), value(&token.text)) {
            if let Some(tens) = value(&last.text) {
                if tens >= 20 && tens % 10 == 0 && tens < 100 && (1..10).contains(&unit) {
                    last.text = format!("#{}", tens + unit);
                    continue;
                }
            }
        }
        joined.push(token);
    }
    joined
}

fn value(token: &str) -> Option<u32> {
    token.strip_prefix('#')?.parse().ok()
}

/// A number written as digits, a word, an ordinal, or (where `roman_ok`) a
/// Roman numeral
fn number(word: &str, roman_ok: bool) -> Option<u32> {
    if let Ok(n) = word.parse::<u32>() {
        return Some(n);
    }
    let digits = word.trim_end_matches(|c: char| c.is_ascii_alphabetic());
    if !digits.is_empty() && matches!(&word[digits.len()..], "st" | "nd" | "rd" | "th") {
        return digits.parse().ok();
    }
    const UNITS: &[&str] = &[
        "zero",
        "one",
        "two",
        "three",
        "four",
        "five",
        "six",
        "seven",
        "eight",
        "nine",
        "ten",
        "eleven",
        "twelve",
        "thirteen",
        "fourteen",
        "fifteen",
        "sixteen",
        "seventeen",
        "eighteen",
        "nineteen",
    ];
    const ORDINALS: &[&str] = &[
        "zeroth",
        "first",
        "second",
        "third",
        "fourth",
        "fifth",
        "sixth",
        "seventh",
        "eighth",
        "ninth",
        "tenth",
        "eleventh",
        "twelfth",
        "thirteenth",
        "fourteenth",
        "fifteenth",
        "sixteenth",
        "seventeenth",
        "eighteenth",
        "nineteenth",
    ];
    const TENS: &[&str] = &[
        "", "", "twenty", "thirty", "forty", "fifty", "sixty", "seventy", "eighty", "ninety",
    ];
    const TENS_ORDINAL: &[&str] = &[
        "",
        "",
        "twentieth",
        "thirtieth",
        "fortieth",
        "fiftieth",
        "sixtieth",
        "seventieth",
        "eightieth",
        "ninetieth",
    ];
    if let Some(n) = UNITS.iter().position(|w| *w == word) {
        return Some(n as u32);
    }
    if let Some(n) = ORDINALS.iter().position(|w| *w == word) {
        return Some(n as u32);
    }
    if let Some(n) = TENS.iter().position(|w| !w.is_empty() && *w == word) {
        return Some(n as u32 * 10);
    }
    if let Some(n) = TENS_ORDINAL
        .iter()
        .position(|w| !w.is_empty() && *w == word)
    {
        return Some(n as u32 * 10);
    }
    if roman_ok {
        return roman(word);
    }
    None
}

/// "iv" → 4, up to 399; `None` for anything that is not a numeral
fn roman(word: &str) -> Option<u32> {
    let digit = |c: char| match c {
        'i' => Some(1),
        'v' => Some(5),
        'x' => Some(10),
        'l' => Some(50),
        'c' => Some(100),
        _ => None,
    };
    let values: Vec<u32> = word.chars().map(digit).collect::<Option<_>>()?;
    let mut total = 0;
    for (i, v) in values.iter().enumerate() {
        if values.get(i + 1).is_some_and(|next| next > v) {
            total -= *v as i32;
        } else {
            total += *v as i32;
        }
    }
    (total > 0 && total < 400).then_some(total as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Words spoken one after another, `gap_ms` apart, each 400 ms long;
    /// "|" adds a one-second silence
    fn speech(text: &str, from_ms: u64) -> Vec<Word> {
        let mut at = from_ms;
        let mut words = Vec::new();
        for part in text.split_whitespace() {
            if part == "|" {
                at += 1_000;
                continue;
            }
            words.push(Word {
                text: part.to_string(),
                start_ms: at,
                end_ms: at + 400,
            });
            at += 420;
        }
        words
    }

    fn start_of(words: &[Word], text: &str) -> u64 {
        words
            .iter()
            .rfind(|w| w.text == text)
            .map(|w| w.start_ms)
            .unwrap()
    }

    #[test]
    fn a_spoken_announcement_matches_its_title() {
        let words = speech(
            "and that was the end. | Chapter Four. Outlaws. In the spring",
            0,
        );
        assert_eq!(
            find_title("Chapter 4: Outlaws", &words, 2_000),
            Some(start_of(&words, "Chapter"))
        );
    }

    #[test]
    fn a_number_written_another_way_still_matches() {
        let words = speech("over. | Part two. Right here, right now. The first", 10_000);
        assert_eq!(
            find_title("Part II: Right Here, Right Now", &words, 10_000),
            Some(start_of(&words, "Part"))
        );
    }

    #[test]
    fn the_number_alone_is_enough_at_the_start_of_a_sentence() {
        let words = speech("they left. | Chapter twelve. | It rained", 0);
        assert_eq!(
            find_title("Chapter 12: The Long March", &words, 0),
            Some(start_of(&words, "Chapter"))
        );
    }

    #[test]
    fn a_misheard_title_word_is_carried_by_the_number() {
        // What Whisper made of "Chapter four. Outlaws." in a test recording
        let words = speech("the last chapter, chapter 4, August.", 0);
        assert_eq!(
            find_title("Chapter 4: Outlaws", &words, 0),
            Some(start_of(&words, "chapter"))
        );
    }

    #[test]
    fn a_number_heard_without_its_division_word_starts_the_title() {
        // What Whisper made of "…military use? [3 s] Chapter 10. Tomorrow's
        // World." in The Future of Geography: "Chapter" was dropped
        let at = |text: &str, start_ms: u64, end_ms: u64| Word {
            text: text.into(),
            start_ms,
            end_ms,
        };
        let words = [
            at("military", 15_360, 17_290),
            at("use?", 15_800, 19_440),
            at("10.", 22_040, 23_300),
            at("Tomorrow's", 23_300, 25_080),
            at("World", 24_040, 26_380),
            at("For", 26_360, 26_640),
        ];
        assert_eq!(
            find_title("Chapter 10: Tomorrow's World", &words, 20_000),
            Some(22_040)
        );
        // Another chapter's number before the words is not taken in
        let words = [
            at("Chapter", 10_000, 10_400),
            at("9.", 10_500, 11_000),
            at("Tomorrow's", 11_500, 12_000),
            at("World", 12_100, 12_500),
        ];
        assert_eq!(
            find_title("Chapter 10: Tomorrow's World", &words, 10_000),
            Some(11_500)
        );
    }

    #[test]
    fn a_misheard_lead_in_is_reported() {
        // What Whisper made of "Chapter 5. China…" in The Future of Geography
        let words = speech("a jungle. | Number 5 China the Long March into space.", 0);
        assert_eq!(
            find_announcement("Chapter 5: China: The Long March Into Space", &words, 0),
            Some(HeardTitle {
                start_ms: start_of(&words, "China"),
                lead_in_unheard: true,
            })
        );
        // Heard in full, or a title with no number, has nothing unheard
        let words = speech("a jungle. | Chapter 5. China, the Long March", 0);
        assert!(
            !find_announcement("Chapter 5: China: The Long March Into Space", &words, 0)
                .unwrap()
                .lead_in_unheard
        );
        let words = speech("the end. | Epilogue. When we", 0);
        assert!(
            !find_announcement("Epilogue", &words, 0)
                .unwrap()
                .lead_in_unheard
        );
    }

    #[test]
    fn a_quote_mark_from_whisper_counts_as_a_break() {
        // What Whisper made of a real recording: a lone quote before "Chapter"
        let mut words = speech("that was the end of the old life", 0);
        words.push(Word {
            text: "\"".into(),
            start_ms: 6_000,
            end_ms: 7_000,
        });
        words.extend(speech("Chapter 4, \"Auglass, in the spring", 7_000));
        assert_eq!(find_title("Chapter 4: Outlaws", &words, 9_000), Some(7_000));
    }

    #[test]
    fn credits_and_unrelated_speech_do_not_match() {
        let words = speech("This is Audible. | Opening", 0);
        assert_eq!(find_title("Opening Credits", &words, 0), None);
        let words = speech("The rain kept falling all night.", 0);
        assert_eq!(find_title("Chapter 4: Outlaws", &words, 0), None);
    }

    #[test]
    fn a_passing_mention_does_not_beat_the_announcement() {
        // The mention is nearer the estimate; the announcement is the chapter
        let mut words = speech("as we saw in chapter four the men", 10_000);
        words.extend(speech("ended. | Chapter Four. Outlaws. The", 40_000));
        assert_eq!(
            find_title("Chapter 4: Outlaws", &words, 12_000),
            Some(start_of(&words, "Chapter"))
        );
    }

    #[test]
    fn a_passing_mention_alone_is_not_heard() {
        let words = speech("as we saw in chapter four the men", 0);
        assert_eq!(find_title("Chapter 4: Outlaws", &words, 0), None);
    }

    #[test]
    fn a_title_without_a_number_matches_on_its_words() {
        let words = speech("fine. | Introduction. | When I first", 0);
        assert_eq!(
            find_title("Introduction", &words, 0),
            Some(start_of(&words, "Introduction."))
        );
        let words = speech("yes. | The Road to the Heavens. It began", 0);
        assert_eq!(
            find_title("Chapter 2: The Road to the Heavens", &words, 0),
            Some(start_of(&words, "Road"))
        );
    }

    #[test]
    fn numbers_normalize_in_every_form() {
        let texts = |t: &str| -> Vec<String> { normalize(t).into_iter().map(|t| t.text).collect() };
        assert_eq!(texts("Chapter 21"), ["chapter", "#21"]);
        assert_eq!(texts("chapter twenty-one"), ["chapter", "#21"]);
        assert_eq!(texts("Chapter Twenty One"), ["chapter", "#21"]);
        assert_eq!(texts("the fourth part"), ["the", "#4", "part"]);
        assert_eq!(
            texts("Part III: The Future"),
            ["part", "#3", "the", "future"]
        );
        assert_eq!(texts("IV"), ["#4"]);
        assert_eq!(texts("I said"), ["i", "said"]);
        assert_eq!(texts("Tomorrow's World"), ["tomorrows", "world"]);
        assert_eq!(texts("1st"), ["#1"]);
    }
}
