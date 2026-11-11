use super::TextFragment;

pub(super) fn reconstruct(
    fragments: &[TextFragment],
    space_thresholds: &[f64],
    merge_hyphenated: bool,
) -> String {
    let mut order: Vec<usize> = (0..fragments.len()).collect();
    order.sort_by(|&a, &b| fragments[b].y.total_cmp(&fragments[a].y));
    let mut result = String::new();
    let mut start = 0;
    while start < order.len() {
        let head = &fragments[order[start]];
        let mut end = start + 1;
        while end < order.len() {
            let fragment = &fragments[order[end]];
            let tolerance = (0.2 * head.height.min(fragment.height)).max(super::SAME_LINE_EPS);
            if (head.y - fragment.y).abs() > tolerance {
                break;
            }
            end += 1;
        }
        let line = &mut order[start..end];
        let rtl = line.iter().any(|&i| fragments[i].text.chars().any(|c|
            matches!(c as u32, 0x0590..=0x08ff | 0xfb1d..=0xfdff | 0xfe70..=0xfeff | 0x10800..=0x10fff | 0x1e800..=0x1eeff)));
        if rtl {
            line.sort_unstable();
        } else {
            line.sort_by(|&a, &b| fragments[a].x.total_cmp(&fragments[b].x));
        }
        if !result.is_empty() {
            let next = &fragments[line[0]].text;
            let fusion = if merge_hyphenated {
                super::hyphen_fusion_action(&result, next)
            } else {
                super::HyphenFusionAction::NoFusion
            };
            match (super::hyphen_wrap_prefix_len(&result), fusion) {
                (Some(prefix), super::HyphenFusionAction::DropHyphen) => result.truncate(prefix),
                (Some(prefix), super::HyphenFusionAction::KeepHyphen) => {
                    result.truncate(prefix + 1)
                }
                _ => result.push('\n'),
            }
        }
        let mut previous: Option<&TextFragment> = None;
        for &i in line.iter() {
            let fragment = &fragments[i];
            if let Some(prev) = previous {
                let gap = if rtl {
                    if fragment.x < prev.x {
                        prev.x - (fragment.x + fragment.width)
                    } else {
                        fragment.x - (prev.x + prev.width)
                    }
                } else {
                    fragment.x - (prev.x + prev.width)
                };
                if gap > space_thresholds[i]
                    && !result.ends_with(char::is_whitespace)
                    && !fragment.text.starts_with(char::is_whitespace)
                {
                    result.push(' ');
                }
            }
            result.push_str(&fragment.text);
            previous = Some(fragment);
        }
        start = end;
    }
    result
}
