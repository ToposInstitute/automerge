use automerge::transaction::{PopulateValue, Transactable};
use automerge::{Automerge, ObjId, ObjType, ScalarValue, ROOT};
use criterion::{criterion_group, criterion_main, Criterion};
use serde_json::Value;
use std::hint::black_box;

fn load_json() -> Value {
    let path = format!("{}/benches/example.json", env!("CARGO_MANIFEST_DIR"));
    let json_str = std::fs::read_to_string(path).expect("cannot read example.json");
    serde_json::from_str(&json_str).expect("cannot parse example.json")
}

// --- Individual-op approach (the slow path) ---

fn insert_value_into_map(
    tx: &mut automerge::transaction::Transaction<'_>,
    parent: &ObjId,
    key: &str,
    value: &Value,
) {
    match value {
        Value::String(s) => {
            let text_id = tx.put_object(parent, key, ObjType::Text).unwrap();
            tx.splice_text(&text_id, 0, 0, s.as_str()).unwrap();
        }
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                tx.put(parent, key, i).unwrap();
            } else if let Some(f) = n.as_f64() {
                tx.put(parent, key, f).unwrap();
            }
        }
        Value::Bool(b) => {
            tx.put(parent, key, *b).unwrap();
        }
        Value::Null => {
            tx.put(parent, key, ()).unwrap();
        }
        Value::Object(map) => {
            let obj_id = tx.put_object(parent, key, ObjType::Map).unwrap();
            for (nested_key, nested_val) in map {
                insert_value_into_map(tx, &obj_id, nested_key.as_str(), nested_val);
            }
        }
        Value::Array(arr) => {
            let list_id = tx.put_object(parent, key, ObjType::List).unwrap();
            for (i, item) in arr.iter().enumerate() {
                insert_value_into_list(tx, &list_id, i, item);
            }
        }
    }
}

fn insert_value_into_list(
    tx: &mut automerge::transaction::Transaction<'_>,
    parent: &ObjId,
    index: usize,
    value: &Value,
) {
    match value {
        Value::String(s) => {
            let text_id = tx.insert_object(parent, index, ObjType::Text).unwrap();
            tx.splice_text(&text_id, 0, 0, s.as_str()).unwrap();
        }
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                tx.insert(parent, index, i).unwrap();
            } else if let Some(f) = n.as_f64() {
                tx.insert(parent, index, f).unwrap();
            }
        }
        Value::Bool(b) => {
            tx.insert(parent, index, *b).unwrap();
        }
        Value::Null => {
            tx.insert(parent, index, ()).unwrap();
        }
        Value::Object(map) => {
            let obj_id = tx.insert_object(parent, index, ObjType::Map).unwrap();
            for (nested_key, nested_val) in map {
                insert_value_into_map(tx, &obj_id, nested_key.as_str(), nested_val);
            }
        }
        Value::Array(arr) => {
            let list_id = tx.insert_object(parent, index, ObjType::List).unwrap();
            for (i, item) in arr.iter().enumerate() {
                insert_value_into_list(tx, &list_id, i, item);
            }
        }
    }
}

fn populate_doc_individual(json: &Value) -> Automerge {
    let mut doc = Automerge::new();
    doc.transact(|tx| {
        let Value::Object(map) = json else {
            panic!("expected object at root");
        };
        for (key, val) in map {
            insert_value_into_map(tx, &ROOT, key.as_str(), val);
        }
        Ok::<_, automerge::AutomergeError>(())
    })
    .unwrap();
    doc
}

// --- populate_map approach (the fast path) ---

fn json_to_populate_value(value: &Value) -> PopulateValue {
    match value {
        Value::String(s) => PopulateValue::Text(s.clone()),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                PopulateValue::Scalar(ScalarValue::Int(i))
            } else if let Some(f) = n.as_f64() {
                PopulateValue::Scalar(ScalarValue::F64(f))
            } else {
                PopulateValue::Scalar(ScalarValue::Null)
            }
        }
        Value::Bool(b) => PopulateValue::Scalar(ScalarValue::Boolean(*b)),
        Value::Null => PopulateValue::Scalar(ScalarValue::Null),
        Value::Object(map) => {
            let entries: Vec<(String, PopulateValue)> = map
                .iter()
                .map(|(k, v)| (k.clone(), json_to_populate_value(v)))
                .collect();
            PopulateValue::Map(entries)
        }
        Value::Array(arr) => {
            let items: Vec<PopulateValue> = arr.iter().map(json_to_populate_value).collect();
            PopulateValue::List(items)
        }
    }
}

fn populate_doc_bulk(json: &Value) -> Automerge {
    let mut doc = Automerge::new();
    let Value::Object(map) = json else {
        panic!("expected object at root");
    };
    let entries: Vec<(String, PopulateValue)> = map
        .iter()
        .map(|(k, v)| (k.clone(), json_to_populate_value(v)))
        .collect();
    let mut tx = doc.transaction();
    tx.populate_map(&ROOT, entries).unwrap();
    tx.commit();
    doc
}

fn criterion_benchmark(c: &mut Criterion) {
    let json = load_json();

    c.bench_function("doc_init_individual_ops", |b| {
        b.iter(|| populate_doc_individual(black_box(&json)))
    });

    c.bench_function("doc_init_populate_map", |b| {
        b.iter(|| populate_doc_bulk(black_box(&json)))
    });

    // Also benchmark save/load to ensure the document produced is valid
    let doc = populate_doc_bulk(&json);

    c.bench_function("doc_init_save", |b| b.iter(|| black_box(doc.save())));

    let bytes = doc.save();
    c.bench_function("doc_init_load", |b| {
        b.iter(|| Automerge::load(black_box(&bytes)).unwrap())
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
