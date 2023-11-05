use criterion::{black_box, criterion_group, criterion_main, Criterion};

const SPECIAL_CHARS_STR: &str = ">:@#~?+-/*&|%{}()[]";

fn is_special_str(c: char) -> bool {
    SPECIAL_CHARS_STR.contains(c)
}

const SPECIAL_CHARS_LIST: &[char] = &[
    '>', ':', '@', '#', '~', '?', '+', '-', '/', '*', '&', '|', '%', '{', '}', '(', ')',
];

fn is_special_list(c: char) -> bool {
    SPECIAL_CHARS_LIST.contains(&c)
}

const SPECIAL_CHARS_LIST_ORDERED: &[char] = &[
    '{', '}', '@', '&', '%', '>', ':', '#', '~', '?', '+', '-', '/', '*', '|', '(', ')',
];

fn is_special_list_ordered(c: char) -> bool {
    SPECIAL_CHARS_LIST_ORDERED.contains(&c)
}

fn is_special_match(c: char) -> bool {
    match c {
        '>' | ':' | '@' | '#' | '~' | '?' | '+' | '-' | '/' | '*' | '&' | '|' | '%' | '{' | '}'
        | '(' | ')' => true,
        _ => false,
    }
}

fn is_special_match_ordered(c: char) -> bool {
    match c {
        '{' | '}' | '@' | '&' | '%' | '>' | ':' | '#' | '~' | '?' | '+' | '-' | '/' | '*' | '|'
        | '(' | ')' => true,
        _ => false,
    }
}

fn test(f: fn(char) -> bool) {
    const TEST: &str = include_str!("test_recipe.cook");

    black_box(TEST).chars().for_each(|c| {
        f(c);
    })
}

fn criterion_benchmark(c: &mut Criterion) {
    let mut g = c.benchmark_group("special chars");
    g.bench_function("none", |b| b.iter(|| test(|_| false)));
    g.bench_function("str", |b| b.iter(|| test(is_special_str)));
    g.bench_function("list", |b| b.iter(|| test(is_special_list)));
    g.bench_function("list_ordered", |b| b.iter(|| test(is_special_list_ordered)));
    g.bench_function("match", |b| b.iter(|| test(is_special_match)));
    g.bench_function("match_ordered", |b| {
        b.iter(|| test(is_special_match_ordered))
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
