use std::{collections::BTreeMap, fs, io, path::Path};

use rime_core::engine::Analyzer;
use rime_core::{dictionary::Dictionary, model::Candidate};
use rime_pinyin::QuanpinPreeditor;

#[derive(Debug, Clone)]
struct Entry {
    text: String,
    weight: i32,
}

/// TSV 格式（简化版）：
///
/// - `text<TAB>key<TAB>weight`
/// - weight 可省略，默认 0
/// - 允许 `#` 开头注释行
///
/// key 建议用“无分隔的拼音串”（例如 `nihao`），与 CLI 输入一致。
pub struct TsvDictionary {
    map: BTreeMap<String, Vec<Entry>>,
    initials_map: BTreeMap<String, Vec<(String, Entry)>>, // initials -> [(key, entry)]
}

impl TsvDictionary {
    pub fn from_path(path: impl AsRef<Path>) -> io::Result<Self> {
        let s = fs::read_to_string(path)?;
        Self::from_tsv_str(&s)
    }

    pub fn from_tsv_str(s: &str) -> io::Result<Self> {
        let mut map: BTreeMap<String, Vec<Entry>> = BTreeMap::new();
        let mut initials_map: BTreeMap<String, Vec<(String, Entry)>> = BTreeMap::new();
        let syllabifier = QuanpinPreeditor::new();

        for (idx, line) in s.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let mut it = line.split('\t');
            let text = it.next().unwrap_or("").trim();
            let key = it.next().unwrap_or("").trim();
            if text.is_empty() || key.is_empty() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("TSV 第 {} 行缺少 text/key", idx + 1),
                ));
            }
            let weight = it
                .next()
                .map(str::trim)
                .filter(|x| !x.is_empty())
                .and_then(|x| x.parse::<i32>().ok())
                .unwrap_or(0);
            let entry = Entry {
                text: text.to_string(),
                weight,
            };
            map.entry(key.to_string()).or_default().push(entry.clone());

            // 预计算：key(如 qishi) -> 音节段(如 [qi, shi]) -> initials(如 qs)
            let analysis = syllabifier.analyze(key);
            if !analysis.segment.is_empty() {
                let mut initials = String::new();
                for seg in &analysis.segment {
                    if let Some(ch) = seg.chars().next() {
                        initials.push(ch);
                    }
                }
                if !initials.is_empty() {
                    initials_map
                        .entry(initials)
                        .or_default()
                        .push((key.to_string(), entry));
                }
            }
        }

        for v in map.values_mut() {
            v.sort_by(|a, b| b.weight.cmp(&a.weight).then_with(|| a.text.cmp(&b.text)));
        }

        for v in initials_map.values_mut() {
            v.sort_by(|a, b| b.1.weight.cmp(&a.1.weight).then_with(|| a.1.text.cmp(&b.1.text)));
        }

        Ok(Self { map, initials_map })
    }

    fn prefix_candidates(
        &self,
        prefix: &str,
        start: usize,
        end: usize,
        limit: usize,
        out: &mut Vec<Candidate>,
    ) {
        if prefix.is_empty() || limit == 0 {
            return;
        }
        for (key, entries) in self.map.range(prefix.to_string()..) {
            if !key.starts_with(prefix) {
                break;
            }
            if key == prefix {
                continue;
            }
            for e in entries {
                out.push(Candidate {
                    text: e.text.clone(),
                    comment: Some(key.clone()),
                    weight: e.weight,
                    segment_start: start,
                    segment_end: end,
                });
                if out.len() >= limit {
                    return;
                }
            }
        }
    }
}

impl Dictionary for TsvDictionary {
    fn lookup_span(
        &self,
        segments: &[String],
        start: usize,
        end: usize,
        limit: usize,
    ) -> Vec<Candidate> {
        let limit = limit.max(1);
        if start >= end || end > segments.len() {
            return Vec::new();
        }

        let key: String = segments[start..end].concat();
        if key.is_empty() {
            return Vec::new();
        }

        let mut out = Vec::new();
        if let Some(entries) = self.map.get(&key) {
            for e in entries.iter().take(limit) {
                out.push(Candidate {
                    text: e.text.clone(),
                    comment: None,
                    weight: e.weight,
                    segment_start: start,
                    segment_end: end,
                });
            }
        }

        // 仅对“整段输入”提供前缀补全（用于 CLI 输入体验）。
        if start == 0 && end == segments.len() && out.len() < limit {
            self.prefix_candidates(&key, start, end, limit - out.len(), &mut out);
        }

        // initials 查询：如果 segments 看起来是 ["q","s"] 这种单字母数组，则尝试用 initials_map。
        if out.is_empty()
            && start == 0
            && end == segments.len()
            && segments.iter().all(|s| s.len() == 1 && s.bytes().all(|b| b.is_ascii_lowercase()))
        {
            let initials: String = segments.concat();
            if let Some(v) = self.initials_map.get(&initials) {
                for (k, e) in v.iter().take(limit) {
                    out.push(Candidate {
                        text: e.text.clone(),
                        comment: Some(k.clone()),
                        weight: e.weight,
                        segment_start: start,
                        segment_end: end,
                    });
                }
            }
        }

        out
    }
}
