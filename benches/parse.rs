use criterion::{criterion_group, criterion_main, Criterion};

use cooklang::{parser::PullParser, CooklangParser, Extensions};

const TEST_RECIPE: &str = include_str!("./test_recipe.cook");
const COMPLEX_TEST_RECIPE: &str = include_str!("./complex_test_recipe.cook");

fn canonical(c: &mut Criterion) {
    let mut group = c.benchmark_group("canonical");

    let canonical = CooklangParser::canonical();
    let extended = CooklangParser::extended();

    group.bench_with_input("parse-canonical", TEST_RECIPE, |b, input| {
        b.iter(|| canonical.parse(input).is_valid())
    });
    group.bench_with_input("parse-extended", TEST_RECIPE, |b, input| {
        b.iter(|| extended.parse(input).is_valid())
    });
    group.bench_with_input("tokens-canonical", TEST_RECIPE, |b, input| {
        b.iter(|| PullParser::new(input, Extensions::empty()).count())
    });
    group.bench_with_input("tokens-extended", TEST_RECIPE, |b, input| {
        b.iter(|| PullParser::new(input, Extensions::all()).count())
    });
    group.bench_with_input("meta", TEST_RECIPE, |b, input| {
        b.iter(|| extended.parse_metadata(input).is_valid())
    });
}

fn extended(c: &mut Criterion) {
    let parser = CooklangParser::extended();

    let mut group = c.benchmark_group("extended");

    group.bench_with_input("parse", COMPLEX_TEST_RECIPE, |b, input| {
        b.iter(|| parser.parse(input).is_valid())
    });
    group.bench_with_input("tokens", COMPLEX_TEST_RECIPE, |b, input| {
        b.iter(|| PullParser::new(input, Extensions::all()).count())
    });
}

criterion_group!(benches, canonical, extended);
criterion_main!(benches);
