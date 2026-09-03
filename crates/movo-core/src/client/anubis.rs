use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::fmt::Write;
use std::time::{Duration, Instant};

/// Anubis proof-of-work algorithm as advertised in the challenge `rules`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum AnubisAlgorithm {
    #[default]
    Fast,
    Slow,
    /// No proof-of-work: the interstitial only requires waiting out a delay.
    #[serde(alias = "metarefresh")]
    Metarefresh,
}

#[derive(Debug, Default, Deserialize)]
pub struct AnubisRules {
    pub algorithm: Option<AnubisAlgorithm>,
    pub difficulty: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct AnubisChallengeInner {
    pub id: String,
    #[serde(rename = "randomData")]
    pub random_data: String,
    #[serde(default)]
    pub difficulty: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct AnubisChallengeData {
    #[serde(default)]
    pub rules: Option<AnubisRules>,
    #[serde(default)]
    pub challenge: Option<AnubisChallengeInner>,
}

#[derive(Debug, Clone)]
pub struct AnubisSolution {
    pub pass_url: String,
    pub challenge_id: String,
    pub algorithm: AnubisAlgorithm,
    /// SHA-256 hex digest for `fast`/`slow` PoW (empty for `metarefresh`).
    pub hash: String,
    /// Nonce for `fast`/`slow` PoW (ignored for `metarefresh`).
    pub nonce: u64,
    pub elapsed_ms: u128,
    pub redir_url: String,
}

/// Name of the cookie Anubis sets to the challenge id; the pass-challenge
/// request MUST echo it back.
pub const VERIFICATION_COOKIE: &str = "techaro.lol-anubis-cookie-verification";

pub fn is_challenge(html: &str) -> bool {
    html.contains("anubis_challenge") || html.contains("/.within.website/x/cmd/anubis/")
}

/// Parses the challenge and computes the solution. For `metarefresh`
/// challenges there is no PoW — Anubis only requires waiting out
/// `(difficulty + 1)` seconds before pass-challenge.
pub async fn extract_and_solve(
    base_origin: &str,
    html: &str,
    redir_url: &str,
) -> Result<AnubisSolution, String> {
    let challenge_json = extract_script_tag(html, "anubis_challenge")
        .ok_or_else(|| "Missing anubis_challenge tag in HTML".to_string())?;

    let prefix_json =
        extract_script_tag(html, "anubis_base_prefix").unwrap_or_else(|| "\"\"".to_string());

    let challenge_data: AnubisChallengeData = serde_json::from_str(&challenge_json)
        .map_err(|e| format!("Failed to parse anubis challenge JSON: {}", e))?;

    let prefix: String = serde_json::from_str(&prefix_json).unwrap_or_default();

    let inner = challenge_data
        .challenge
        .ok_or_else(|| "anubis challenge missing 'challenge' object".to_string())?;
    let rules = challenge_data.rules.unwrap_or_default();

    let random_data = &inner.random_data;
    let challenge_id = inner.id.clone();
    let difficulty = rules.difficulty.or(inner.difficulty).unwrap_or(0);
    let algorithm = rules.algorithm.unwrap_or_default();

    let start = Instant::now();
    let (nonce, hash) = match algorithm {
        AnubisAlgorithm::Fast | AnubisAlgorithm::Slow => solve_pow(random_data, difficulty),
        AnubisAlgorithm::Metarefresh => {
            let wait_ms = (difficulty + 1) as u64 * 1000;
            log::info!(
                "Anubis metarefresh: waiting {}ms before pass-challenge",
                wait_ms
            );
            tokio::time::sleep(Duration::from_millis(wait_ms)).await;
            (0, String::new())
        }
    };
    let elapsed_ms = start.elapsed().as_millis();

    let pass_url = format!(
        "{}{}/.within.website/x/cmd/anubis/api/pass-challenge",
        base_origin.trim_end_matches('/'),
        prefix
    );

    Ok(AnubisSolution {
        pass_url,
        challenge_id,
        algorithm,
        hash,
        nonce,
        elapsed_ms,
        redir_url: redir_url.to_string(),
    })
}

fn extract_script_tag(html: &str, id: &str) -> Option<String> {
    // templ.JSONScript HTML-escapes `<`, `>`, `&` inside the JSON, so a
    // non-greedy scan up to the closing tag is safe.
    let marker = format!("id=\"{}\"", id);
    let start_pos = html.find(&marker)?;
    let rest = &html[start_pos..];
    let tag_close = rest.find('>')?;
    let content_start = &rest[tag_close + 1..];
    let script_end = content_start.find("</script>")?;
    Some(content_start[..script_end].trim().to_string())
}

fn solve_pow(random_data: &str, difficulty: usize) -> (u64, String) {
    let mut nonce: u64 = 0;
    let mut buffer = String::with_capacity(random_data.len() + 20);

    loop {
        buffer.clear();
        buffer.push_str(random_data);
        let _ = write!(buffer, "{}", nonce);

        let mut hasher = Sha256::new();
        hasher.update(buffer.as_bytes());
        let result = hasher.finalize();
        let hash_hex = hex::encode(result);

        if hash_hex.bytes().take(difficulty).all(|b| b == b'0') {
            return (nonce, hash_hex);
        }

        nonce += 1;
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solves_the_proof_of_work_challenge() {
        let (_nonce, hash) = solve_pow("test_random_data_sample", 2);
        assert!(hash.starts_with("00"));
    }
}
