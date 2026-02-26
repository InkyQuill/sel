//! Benchmark for large file processing with complex selectors.

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use sel::selector::{LineSpec, Selector};

fn bench_selector_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("selector_parsing");

    // Benchmark parsing selectors of different sizes
    let cases = vec![
        ("single", "42"),
        ("small_range", "10-100"),
        ("small_complex", "1,5,10-15,20,25,30-35"),
        ("medium_complex", "1-10,15-20,25-30,35-40,45-50"),
        ("large_mixed", "1,3,5,7,9,11-20,22,24,26,28,30-40,42,44,46,48,50"),
    ];

    for (name, selector) in cases {
        group.bench_with_input(BenchmarkId::from_parameter(name), selector, |b, s| {
            b.iter(|| Selector::parse(black_box(s)))
        });
    }

    group.finish();
}

fn bench_selector_normalize(c: &mut Criterion) {
    let mut group = c.benchmark_group("selector_normalize");

    let cases: Vec<(&str, Selector)> = vec![
        ("single", Selector::LineNumbers(vec![LineSpec::Single(42)])),
        ("small_range", Selector::LineNumbers(vec![LineSpec::Range(1, 100)])),
        ("overlapping", Selector::LineNumbers(vec![
            LineSpec::Range(1, 50),
            LineSpec::Range(30, 80),
            LineSpec::Range(70, 100),
        ])),
        ("adjacent", Selector::LineNumbers(vec![
            LineSpec::Range(1, 10),
            LineSpec::Single(11),
            LineSpec::Range(12, 20),
        ])),
        ("large_mixed", Selector::LineNumbers(vec![
            LineSpec::Single(1),
            LineSpec::Range(3, 10),
            LineSpec::Single(12),
            LineSpec::Range(15, 25),
            LineSpec::Single(27),
            LineSpec::Range(30, 40),
        ])),
    ];

    for (name, selector) in cases {
        group.bench_with_input(BenchmarkId::from_parameter(name), &selector, |b, s| {
            b.iter(|| s.normalize())
        });
    }

    group.finish();
}

fn bench_line_matching(c: &mut Criterion) {
    let mut group = c.benchmark_group("line_matching");

    // Create a normalized selector with overlapping ranges (simulating 1-100)
    let selector = Selector::LineNumbers(vec![
        LineSpec::Range(1, 50),
        LineSpec::Range(30, 80),
        LineSpec::Range(70, 100),
    ]);
    let normalized = selector.normalize();

    let specs = match &normalized {
        Selector::LineNumbers(s) => s,
        _ => panic!("Expected LineNumbers"),
    };

    group.bench_function("check_100_lines_normalized", |b| {
        b.iter(|| {
            for line_no in 1..=100 {
                black_box(specs.iter().any(|s| s.contains(line_no)));
            }
        })
    });

    // Compare with non-normalized selector
    let non_normalized_specs = match &selector {
        Selector::LineNumbers(s) => s,
        _ => panic!("Expected LineNumbers"),
    };

    group.bench_function("check_100_lines_non_normalized", |b| {
        b.iter(|| {
            for line_no in 1..=100 {
                black_box(non_normalized_specs.iter().any(|s| s.contains(line_no)));
            }
        })
    });

    group.finish();
}

fn bench_large_file_processing(c: &mut Criterion) {
    let mut group = c.benchmark_group("large_file");

    group.bench_function("parse_10000_line_selector", |b| {
        let selector_str = "1-100,200-300,500-600,1000-2000,5000-6000,8000-9000";
        b.iter(|| Selector::parse(black_box(selector_str)))
    });

    group.bench_function("normalize_complex_selector", |b| {
        let selector = Selector::parse("1-100,50-150,200-300,250-350,1000-2000").unwrap();
        b.iter(|| black_box(&selector).normalize())
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_selector_parsing,
    bench_selector_normalize,
    bench_line_matching,
    bench_large_file_processing
);
criterion_main!(benches);
