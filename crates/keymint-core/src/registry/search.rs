use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config as MatcherConfig, Matcher, Utf32Str};

use super::manifest::CommandManifest;

/// Fuzzy score of `query` against a command, or `None` when it does not match.
/// Title matches count most, then keywords, then category/subtitle.
pub fn match_score(
    matcher: &mut Matcher,
    pattern: &Pattern,
    m: &CommandManifest,
    buf: &mut Vec<char>,
) -> Option<u32> {
    let mut score = |hay: &str| pattern.score(Utf32Str::new(hay, buf), matcher);
    let title = score(&m.title).map(|s| s * 4);
    let keywords = if m.keywords.is_empty() {
        None
    } else {
        score(&m.keywords.join(" ")).map(|s| s * 2)
    };
    let other = score(&format!("{} {}", m.category, m.subtitle));
    [title, keywords, other].into_iter().flatten().max()
}

pub fn new_matcher() -> Matcher {
    Matcher::new(MatcherConfig::DEFAULT)
}

pub fn parse_pattern(query: &str) -> Pattern {
    Pattern::parse(query, CaseMatching::Ignore, Normalization::Smart)
}
