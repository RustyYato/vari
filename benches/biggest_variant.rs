use criterion::{black_box, criterion_group, criterion_main, Criterion};
use vari::{tlist, Vari};

criterion_group!(benches, init);
criterion_main!(benches);

type Uniform = Vari<tlist!(i32, u32, f32)>;
type NonUniform = Vari<tlist!(i32, u32, [f32; 10])>;

pub fn init(c: &mut Criterion) {
    c.bench_function("bv uniform alloc init", |b| {
        b.iter(|| Uniform::new(black_box(20_u32)))
    });
    c.bench_function("bv non-uniform sized alloc init", |b| {
        b.iter(|| NonUniform::new(black_box(20_u32)))
    });
    let mut vari = Uniform::new(20_u32);
    c.bench_function("bv uniform alloc set", |b| {
        b.iter(|| vari.set(black_box(20_u32)))
    });
    let mut vari = NonUniform::new(20_u32);
    c.bench_function("bv non-uniform alloc set", |b| {
        b.iter(|| vari.set(black_box(20_u32)))
    });
    let mut vari = NonUniform::new(20_u32);
    c.bench_function("bv non-uniform alloc set large", |b| {
        b.iter(|| {
            vari.set(black_box([
                0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0,
            ]))
        })
    });
}
