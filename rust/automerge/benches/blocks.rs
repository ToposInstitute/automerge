use automerge::{transaction::Transactable, Automerge, ObjType, ReadDoc, ROOT};
use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;

fn load_paragraphs() -> Vec<String> {
    let path = format!(
        "{}/benches/war_and_peace_500.txt",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read_to_string(path)
        .expect("cannot read war_and_peace_500.txt")
        .lines()
        .map(String::from)
        .collect()
}

fn build_block_doc(paragraphs: &[String]) -> Automerge {
    let mut doc = Automerge::new();
    let mut txn = doc.transaction();
    let text = txn.put_object(ROOT, "mynote", ObjType::Text).unwrap();

    for (i, para) in paragraphs.iter().enumerate() {
        let len = txn.length(&text);
        if len > 0 {
            let block = txn.split_block(&text, len).unwrap();
            txn.put(&block, "type", "paragraph").unwrap();
        }
        txn.splice_text(&text, txn.length(&text), 0, para).unwrap();

        if (i + 1) % 50 == 0 {
            txn.commit();
            txn = doc.transaction();
        }
    }
    txn.commit();
    doc
}

fn criterion_benchmark(c: &mut Criterion) {
    let paragraphs = load_paragraphs();

    c.bench_function("block_text_insert_500", |b| {
        b.iter(|| build_block_doc(black_box(&paragraphs)))
    });

    let doc = build_block_doc(&paragraphs);

    c.bench_function("block_text_save_500", |b| b.iter(|| black_box(doc.save())));

    let bytes = doc.save();
    c.bench_function("block_text_load_500", |b| {
        b.iter(|| Automerge::load(black_box(&bytes)).unwrap())
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
