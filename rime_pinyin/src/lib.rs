//! 全拼（quanpin）相关：第一版只做“切分 + preedit 展示”。

use rime_core::engine::{Analysis, Analyzer};

include!(concat!(env!("OUT_DIR"), "/syllabary_gen.rs"));

pub struct QuanpinPreeditor {
    syllables: Vec<(&'static str, i32)>,
}

impl Default for QuanpinPreeditor {
    fn default() -> Self {
        // 按频次降序，遇到同长/同分时更稳定；
        // 但 DP 里我们仍会对“长音节”给更高的结构性分数。
        let mut syllables = SYLLABARY.to_vec();
        syllables.sort_by(|a, b| {
            b.1.cmp(&a.1)
                .then_with(|| b.0.len().cmp(&a.0.len()))
                .then_with(|| a.0.cmp(b.0))
        });
        Self { syllables }
    }
}

impl QuanpinPreeditor {
    pub fn new() -> Self {
        Self::default()
    }

    fn segment_chunk(&self, chunk: &str) -> Option<Vec<&'static str>> {
        if chunk.is_empty() {
            return Some(Vec::new());
        }
        if !chunk.bytes().all(|b| b.is_ascii_lowercase()) {
            return None;
        }

        let n = chunk.len();
        let mut best_score: Vec<Option<i64>> = vec![None; n + 1];
        let mut prev: Vec<Option<(usize, &'static str, i32)>> = vec![None; n + 1];
        best_score[0] = Some(0);

        for i in 0..n {
            let Some(base) = best_score[i] else { continue };
            let rest = &chunk[i..];
            // 遍历所有可能音节：第一版简单暴力；n 一般很小。
            for &(sy, freq) in &self.syllables {
                if !rest.starts_with(sy) {
                    continue;
                }
                let j = i + sy.len();
                // 结构分：优先长音节，辅以频次
                let score = base + (sy.len() as i64) * 10_000 + (freq as i64);
                if best_score[j].is_none() || score > best_score[j].unwrap() {
                    best_score[j] = Some(score);
                    prev[j] = Some((i, sy, freq));
                }
            }
        }

        if best_score[n].is_none() {
            return None;
        }

        // 回溯
        let mut out = Vec::new();
        let mut cur = n;
        while cur > 0 {
            let Some((p, sy, _freq)) = prev[cur] else {
                return None;
            };
            out.push(sy);
            cur = p;
        }
        out.reverse();
        Some(out)
    }

    fn segment(&self, input: &str) -> Option<Vec<&'static str>> {
        // 支持用 `'` 强制断开（Rime 常用来消歧/断词）。
        let mut out = Vec::new();
        for chunk in input.split('\'') {
            let mut seg = self.segment_chunk(chunk)?;
            out.append(&mut seg);
        }
        Some(out)
    }
}

impl Analyzer for QuanpinPreeditor {
    fn analyze(&self, input: &str) -> Analysis {
        if input.is_empty() {
            return Analysis {
                segment: Vec::new(),
                preedit: String::new(),
            };
        }
        let input = input.to_ascii_lowercase();
        match self.segment(&input) {
            Some(segs) if !segs.is_empty() => Analysis {
                preedit: segs.join(" "),
                segment: segs.iter().map(|s| (*s).to_string()).collect(),
            },
            _ => {
                // initials 模式：当无法切分成合法音节时，退化为“按字母段”。
                // 例如输入 `qs` -> segments ["q", "s"]，便于词典做首字母检索。
                let letters_only = input.chars().all(|c| c.is_ascii_lowercase() || c == '\'');
                if letters_only && (1..=6).contains(&input.len()) {
                    let segments: Vec<String> = input
                        .chars()
                        .filter(|&c| c != '\'')
                        .map(|c| c.to_string())
                        .collect();
                    Analysis {
                        preedit: segments.join(" "),
                        segment: segments,
                    }
                } else {
                    Analysis {
                        segment: Vec::new(),
                        preedit: input,
                    }
                }
            }
        }
    }
}
