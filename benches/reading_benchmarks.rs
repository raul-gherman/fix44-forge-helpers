use criterion::{
    Criterion, //
    criterion_group,
    criterion_main,
};
use fix44_forge_helpers::*;
use std::hint::black_box;

fn benchmark_reading_integers(c: &mut Criterion) {
    let mut group = c.benchmark_group("reading_integers");

    // u16 benchmarks
    group.bench_function("read_u16_small", |b| {
        b.iter(|| read_u16(black_box(b"123")))
    });

    group.bench_function("read_u16_max", |b| {
        b.iter(|| read_u16(black_box(b"65535")))
    });

    // u32 benchmarks
    group.bench_function("read_u32_small", |b| {
        b.iter(|| read_u32(black_box(b"12345")))
    });

    group.bench_function("read_u32_large", |b| {
        b.iter(|| read_u32(black_box(b"4294967295")))
    });

    // u64 benchmarks
    group.bench_function("read_u64_small", |b| {
        b.iter(|| read_u64(black_box(b"123456789")))
    });

    group.bench_function("read_u64_large", |b| {
        b.iter(|| {
            read_u64(black_box(
                b"18446744073709551615",
            ))
        })
    });

    // Signed integer benchmarks
    group.bench_function("read_i32_positive", |b| {
        b.iter(|| read_i32(black_box(b"123456789")))
    });

    group.bench_function("read_i32_negative", |b| {
        b.iter(|| read_i32(black_box(b"-123456789")))
    });

    group.bench_function("read_i64_positive", |b| {
        b.iter(|| {
            read_i64(black_box(
                b"9223372036854775807",
            ))
        })
    });

    group.bench_function("read_i64_negative", |b| {
        b.iter(|| {
            read_i64(black_box(
                b"-9223372036854775808",
            ))
        })
    });

    group.finish();
}

fn benchmark_reading_floats(c: &mut Criterion) {
    let mut group = c.benchmark_group("reading_floats");

    group.bench_function("read_f32_integer", |b| {
        b.iter(|| read_f32(black_box(b"123")))
    });

    group.bench_function("read_f32_decimal", |b| {
        b.iter(|| read_f32(black_box(b"123.456789")))
    });

    group.bench_function("read_f32_negative", |b| {
        b.iter(|| read_f32(black_box(b"-123.456789")))
    });

    group.bench_function("read_f64_integer", |b| {
        b.iter(|| read_f64(black_box(b"123456789")))
    });

    group.bench_function("read_f64_decimal", |b| {
        b.iter(|| read_f64(black_box(b"123.456789012345")))
    });

    group.bench_function("read_f64_negative", |b| {
        b.iter(|| {
            read_f64(black_box(
                b"-123.456789012345",
            ))
        })
    });

    group.finish();
}

fn benchmark_reading_other(c: &mut Criterion) {
    let mut group = c.benchmark_group("reading_other");

    group.bench_function("read_bool_true", |b| {
        b.iter(|| read_bool(black_box(b"Y")))
    });

    group.bench_function("read_bool_false", |b| {
        b.iter(|| read_bool(black_box(b"N")))
    });

    group.bench_function("read_str_short", |b| {
        b.iter(|| read_str(black_box(b"MSFT")))
    });

    group.bench_function("read_str_long", |b| {
        b.iter(|| {
            read_str(black_box(
                b"SOME_VERY_LONG_SYMBOL_NAME_FOR_TESTING",
            ))
        })
    });

    group.finish();
}

fn benchmark_mixed_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("mixed_parsing");

    // Simulate parsing multiple fields as might occur in a FIX message
    group.bench_function("parse_fix_like_fields", |b| {
        b.iter(|| {
            let _ = read_u32(black_box(b"34")); // MsgSeqNum
            let _ = read_str(black_box(b"D")); // MsgType
            let _ = read_f64(black_box(b"123.45")); // Price
            let _ = read_u64(black_box(b"1000")); // OrderQty
            let _ = read_str(black_box(b"MSFT")); // Symbol
            let _ = read_bool(black_box(b"Y")); // PossDupFlag
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    benchmark_reading_integers,
    benchmark_reading_floats,
    benchmark_reading_other,
    benchmark_mixed_parsing
);
criterion_main!(benches);
