use serde_json::{Value, json};

pub fn load(file: &str) -> Value {
    let path = format!("{}/tests/spec/{file}.json", env!("CARGO_MANIFEST_DIR"));
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

pub fn core() -> Value {
    load("core-api-2026-04-16")
}

pub fn connect() -> Value {
    load("oauth-connect-1.0.0")
}

pub fn validator(document: &Value, schema: &str) -> jsonschema::Validator {
    let root = json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "allOf": [{"$ref": format!("#/components/schemas/{schema}")}],
        "components": document["components"],
    });
    jsonschema::validator_for(&root)
        .unwrap_or_else(|error| panic!("{schema} does not compile: {error}"))
}

pub fn assert_valid(document: &Value, schema: &str, instance: &Value) {
    let validator = validator(document, schema);
    let errors: Vec<String> = validator
        .iter_errors(instance)
        .map(|error| format!("{error} at {}", error.instance_path))
        .collect();
    assert!(
        errors.is_empty(),
        "{schema} rejected {instance}:\n{}",
        errors.join("\n")
    );
}

pub fn assert_valid_inline(document: &Value, schema: &Value, instance: &Value) {
    let root = json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "allOf": [schema],
        "components": document["components"],
    });
    let validator = jsonschema::validator_for(&root).expect("the schema compiles");
    let errors: Vec<String> = validator
        .iter_errors(instance)
        .map(|error| format!("{error} at {}", error.instance_path))
        .collect();
    assert!(
        errors.is_empty(),
        "rejected {instance}:\n{}",
        errors.join("\n")
    );
}
