use criterion::{criterion_group, criterion_main, Criterion};

use cooklang::{parser::PullParser, CooklangParser, Extensions};

const TEST_RECIPE: &str = include_str!("./test_recipe.cook");

fn complete_recipe(c: &mut Criterion) {
    let mut group = c.benchmark_group("complete_recipe");

    let canonical = CooklangParser::canonical();
    let extended = CooklangParser::extended();

    group.bench_with_input("cooklang-rs-canonical", TEST_RECIPE, |b, input| {
        b.iter(|| canonical.parse(input))
    });
    group.bench_with_input("cooklang-rs-extended", TEST_RECIPE, |b, input| {
        b.iter(|| extended.parse(input))
    });
}

fn just_events(c: &mut Criterion) {
    let mut group = c.benchmark_group("just_events");

    group.bench_with_input("cooklang-rs-extended", TEST_RECIPE, |b, input| {
        b.iter(|| PullParser::new(input, Extensions::all()).count())
    });
    group.bench_with_input("cooklang-rs-canonical", TEST_RECIPE, |b, input| {
        b.iter(|| PullParser::new(input, Extensions::empty()).count())
    });
}

fn just_metadata(c: &mut Criterion) {
    let mut group = c.benchmark_group("just_metadata");
    let parser = CooklangParser::default();
    group.bench_with_input("cooklang-rs", TEST_RECIPE, |b, input| {
        b.iter(|| parser.parse_metadata(input))
    });
}

criterion_group!(benches, complete_recipe, just_events, just_metadata);
criterion_main!(benches);
