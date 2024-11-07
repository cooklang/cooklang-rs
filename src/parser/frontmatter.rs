#[derive(Debug)]
pub struct FrontMatterSplit<'i> {
    pub yaml_text: &'i str,
    pub yaml_offset: usize,
    pub cooklang_text: &'i str,
    pub cooklang_offset: usize,
}

const YAML_FENCE: &str = "---";

pub fn parse_frontmatter(input: &str) -> Option<FrontMatterSplit> {
    let mut fences = fences(input, YAML_FENCE);
    let (_, yaml_start) = fences.next()?;
    let (yaml_end, cooklang_start) = fences.next()?;
    let yaml_text = &input[yaml_start..yaml_end];
    let cooklang_text = &input[cooklang_start..];
    Some(FrontMatterSplit {
        yaml_text,
        yaml_offset: yaml_start,
        cooklang_text,
        cooklang_offset: cooklang_start,
    })
}

fn lines_with_offset(s: &str) -> impl Iterator<Item = (&str, usize)> {
    let mut offset = 0;
    s.split_inclusive('\n').map(move |l| {
        let l_offset = offset;
        offset += l.len();
        (l, l_offset)
    })
}

fn fences<'a>(s: &'a str, fence: &'static str) -> impl Iterator<Item = (usize, usize)> + 'a {
    let lines = lines_with_offset(s);
    lines.filter_map(move |(line, offset)| {
        if line.trim_end() == fence {
            let fence_start = offset;
            let fence_end = offset + line.len();
            return Some((fence_start, fence_end));
        }
        None
    })
}
