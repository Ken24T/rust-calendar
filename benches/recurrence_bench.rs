// Benchmark for recurrence calculations
// Measures performance of fortnightly and quarterly occurrence generation

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};

// Mock types for demonstration (will use actual types)
#[derive(Debug, Clone, Copy)]
enum Frequency {
    Fortnightly,
    Quarterly,
}

// Mock calculation function
fn calculate_occurrences(freq: Frequency, start_year: i32, count: usize) -> Vec<(i32, u32, u32)> {
    let mut occurrences = Vec::with_capacity(count);
    let mut year = start_year;
    let mut month = 1u32;
    let day = 1u32;
    
    for _ in 0..count {
        occurrences.push((year, month, day));
        
        match freq {
            Frequency::Fortnightly => {
                // Simplified: just increment month for demo
                month += 1;
                if month > 12 {
                    month = 1;
                    year += 1;
                }
            }
            Frequency::Quarterly => {
                month += 3;
                if month > 12 {
                    month -= 12;
                    year += 1;
                }
            }
        }
    }
    
    occurrences
}

fn bench_fortnightly_occurrences(c: &mut Criterion) {
    let mut group = c.benchmark_group("fortnightly_occurrences");
    
    for count in [10, 100, 1000].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(count),
            count,
            |b, &count| {
                b.iter(|| {
                    calculate_occurrences(
                        black_box(Frequency::Fortnightly),
                        black_box(2025),
                        black_box(count),
                    )
                });
            },
        );
    }
    
    group.finish();
}

fn bench_quarterly_occurrences(c: &mut Criterion) {
    let mut group = c.benchmark_group("quarterly_occurrences");
    
    for count in [10, 100, 400].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(count),
            count,
            |b, &count| {
                b.iter(|| {
                    calculate_occurrences(
                        black_box(Frequency::Quarterly),
                        black_box(2025),
                        black_box(count),
                    )
                });
            },
        );
    }
    
    group.finish();
}

fn bench_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("frequency_comparison");
    
    let count = 100;
    
    group.bench_function("fortnightly_100", |b| {
        b.iter(|| {
            calculate_occurrences(
                black_box(Frequency::Fortnightly),
                black_box(2025),
                black_box(count),
            )
        });
    });
    
    group.bench_function("quarterly_100", |b| {
        b.iter(|| {
            calculate_occurrences(
                black_box(Frequency::Quarterly),
                black_box(2025),
                black_box(count),
            )
        });
    });
    
    group.finish();
}

criterion_group!(
    benches,
    bench_fortnightly_occurrences,
    bench_quarterly_occurrences,
    bench_comparison
);
criterion_main!(benches);
