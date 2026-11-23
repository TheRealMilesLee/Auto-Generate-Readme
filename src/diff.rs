//! Line-based unified diff for `readme-gen diff`.

use std::fmt::Write as _;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Op<'a> {
    Eq(&'a str),
    Del(&'a str),
    Add(&'a str),
}

pub fn unified_diff(
    old_label: &str,
    new_label: &str,
    old_text: &str,
    new_text: &str,
    context: usize,
) -> Option<String> {
    if old_text == new_text {
        return None;
    }

    let old_lines: Vec<&str> = old_text.lines().collect();
    let new_lines: Vec<&str> = new_text.lines().collect();

    let ops = compute_ops(&old_lines, &new_lines);
    let hunks = build_hunks(&ops, context);
    if hunks.is_empty() {
        return None;
    }

    let mut out = String::new();
    let _ = writeln!(out, "--- {old_label}");
    let _ = writeln!(out, "+++ {new_label}");
    for hunk in hunks {
        let _ = writeln!(
            out,
            "@@ -{},{} +{},{} @@",
            hunk.old_start, hunk.old_count, hunk.new_start, hunk.new_count
        );
        for op in hunk.ops {
            match op {
                Op::Eq(line) => {
                    let _ = writeln!(out, " {line}");
                }
                Op::Del(line) => {
                    let _ = writeln!(out, "-{line}");
                }
                Op::Add(line) => {
                    let _ = writeln!(out, "+{line}");
                }
            }
        }
    }
    Some(out)
}

fn compute_ops<'a>(old_lines: &[&'a str], new_lines: &[&'a str]) -> Vec<Op<'a>> {
    let mut prefix = 0usize;
    while prefix < old_lines.len()
        && prefix < new_lines.len()
        && old_lines[prefix] == new_lines[prefix]
    {
        prefix += 1;
    }

    let mut suffix = 0usize;
    while suffix < old_lines.len() - prefix
        && suffix < new_lines.len() - prefix
        && old_lines[old_lines.len() - 1 - suffix] == new_lines[new_lines.len() - 1 - suffix]
    {
        suffix += 1;
    }

    let old_mid = &old_lines[prefix..old_lines.len() - suffix];
    let new_mid = &new_lines[prefix..new_lines.len() - suffix];

    let mut ops = Vec::with_capacity(old_lines.len() + new_lines.len());
    for &line in &old_lines[..prefix] {
        ops.push(Op::Eq(line));
    }

    let m = old_mid.len();
    let n = new_mid.len();
    if m == 0 {
        for &line in new_mid {
            ops.push(Op::Add(line));
        }
    } else if n == 0 {
        for &line in old_mid {
            ops.push(Op::Del(line));
        }
    } else if m.saturating_mul(n) > 2_000_000 {
        for &line in old_mid {
            ops.push(Op::Del(line));
        }
        for &line in new_mid {
            ops.push(Op::Add(line));
        }
    } else {
        let cols = n + 1;
        let mut dp = vec![0u32; (m + 1) * cols];
        for i in (0..m).rev() {
            for j in (0..n).rev() {
                dp[i * cols + j] = if old_mid[i] == new_mid[j] {
                    dp[(i + 1) * cols + (j + 1)] + 1
                } else {
                    dp[(i + 1) * cols + j].max(dp[i * cols + (j + 1)])
                };
            }
        }
        let mut i = 0usize;
        let mut j = 0usize;
        while i < m && j < n {
            if old_mid[i] == new_mid[j] {
                ops.push(Op::Eq(old_mid[i]));
                i += 1;
                j += 1;
            } else if dp[(i + 1) * cols + j] >= dp[i * cols + (j + 1)] {
                ops.push(Op::Del(old_mid[i]));
                i += 1;
            } else {
                ops.push(Op::Add(new_mid[j]));
                j += 1;
            }
        }
        while i < m {
            ops.push(Op::Del(old_mid[i]));
            i += 1;
        }
        while j < n {
            ops.push(Op::Add(new_mid[j]));
            j += 1;
        }
    }

    for &line in &old_lines[old_lines.len() - suffix..] {
        ops.push(Op::Eq(line));
    }
    ops
}

struct Hunk<'a> {
    old_start: usize,
    old_count: usize,
    new_start: usize,
    new_count: usize,
    ops: Vec<Op<'a>>,
}

fn build_hunks<'a>(ops: &[Op<'a>], context: usize) -> Vec<Hunk<'a>> {
    let mut old_pos = Vec::with_capacity(ops.len());
    let mut new_pos = Vec::with_capacity(ops.len());
    let mut o = 1usize;
    let mut n = 1usize;
    for op in ops {
        old_pos.push(o);
        new_pos.push(n);
        match op {
            Op::Eq(_) => {
                o += 1;
                n += 1;
            }
            Op::Del(_) => o += 1,
            Op::Add(_) => n += 1,
        }
    }

    let mut change_ranges: Vec<(usize, usize)> = Vec::new();
    let mut idx = 0usize;
    while idx < ops.len() {
        if matches!(ops[idx], Op::Eq(_)) {
            idx += 1;
            continue;
        }
        let start = idx.saturating_sub(context);
        let mut end = idx;
        while end < ops.len() {
            if !matches!(ops[end], Op::Eq(_)) {
                end += 1;
                continue;
            }
            let eq_start = end;
            while end < ops.len() && matches!(ops[end], Op::Eq(_)) {
                end += 1;
            }
            if end < ops.len() && end - eq_start <= context * 2 {
                continue;
            }
            end = (eq_start + context).min(ops.len());
            break;
        }
        change_ranges.push((start, end));
        idx = end;
    }

    change_ranges
        .into_iter()
        .map(|(start, end)| {
            let slice = &ops[start..end];
            let old_count = slice
                .iter()
                .filter(|op| matches!(op, Op::Eq(_) | Op::Del(_)))
                .count();
            let new_count = slice
                .iter()
                .filter(|op| matches!(op, Op::Eq(_) | Op::Add(_)))
                .count();
            Hunk {
                old_start: old_pos[start],
                old_count,
                new_start: new_pos[start],
                new_count,
                ops: slice.to_vec(),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_returns_none() {
        assert!(unified_diff("a", "b", "one\ntwo\n", "one\ntwo\n", 3).is_none());
    }

    #[test]
    fn emits_unified_hunk() {
        let diff = unified_diff(
            "a/README.md",
            "b/README.md",
            "one\ntwo\nthree\n",
            "one\nTWO\nthree\nfour\n",
            2,
        )
        .unwrap();
        assert!(diff.contains("--- a/README.md"));
        assert!(diff.contains("+++ b/README.md"));
        assert!(diff.contains("-two"));
        assert!(diff.contains("+TWO"));
        assert!(diff.contains("+four"));
    }
}
