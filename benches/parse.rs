use criterion::{criterion_group, criterion_main, Criterion};

use cooklang::CooklangParser;

const TEST_RECIPE: &str = include_str!("./test_recipe.cook");
const COMPLEX_TEST_RECIPE: &str = include_str!("./complex_test_recipe.cook");

fn canonical(c: &mut Criterion) {
    let mut group = c.benchmark_group("canonical");

    let canonical = CooklangParser::canonical();
    let extended = CooklangParser::extended();

    group.bench_with_input("cooklang-rs-canonical", TEST_RECIPE, |b, input| {
        b.iter(|| canonical.parse(input))
    });
    group.bench_with_input("cooklang-rs", TEST_RECIPE, |b, input| {
        b.iter(|| extended.parse(input))
    });
    group.bench_with_input("cooklang-rs-meta", TEST_RECIPE, |b, input| {
        b.iter(|| extended.parse_metadata(input))
    });
}

fn extended(c: &mut Criterion) {
    let parser = CooklangParser::extended();

    let mut group = c.benchmark_group("extended");

    group.bench_with_input("cooklang-rs", COMPLEX_TEST_RECIPE, |b, input| {
        b.iter(|| parser.parse(input))
    });
}

criterion_group!(benches, canonical, extended);
criterion_main!(benches);
