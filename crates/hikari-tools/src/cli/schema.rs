use crate::cli::opt;
use schemars::{JsonSchema, schema_for};

pub(crate) fn exec(schema: opt::Schema) -> Result<(), anyhow::Error> {
    let opt::Schema { output_folder } = schema;

    // Store every schema in a file
    // Check of output_folder exists
    if !std::path::Path::new(&output_folder).exists() {
        std::fs::create_dir(&output_folder)?;
    }

    generate_and_store_schema::<hikari_config::global::VersionConfig>(
        "AMSL Global",
        &format!("{output_folder}/global.json"),
    )?;
    generate_and_store_schema::<hikari_config::module::VersionConfig>(
        "AMSL Module",
        &format!("{output_folder}/module.json"),
    )?;
    generate_and_store_schema::<hikari_llm::builder::VersionConfig>(
        "AMSL LLM Agent",
        &format!("{output_folder}/llm_agent.json"),
    )?;
    generate_and_store_schema::<hikari_config::assessment::VersionConfig>(
        "AMSL Assessment",
        &format!("{output_folder}/assessment.json"),
    )?;
    generate_and_store_schema::<hikari_config::documents::VersionConfig>(
        "AMSL Collection",
        &format!("{output_folder}/collection.json"),
    )?;
    generate_and_store_schema::<hikari_config::constants::VersionConfig>(
        "AMSL Constants",
        &format!("{output_folder}/constants.json"),
    )?;

    println!("Generated schemas in {output_folder}");
    Ok(())
}

fn rename_schema(schema: serde_json::Value, title: &str) -> serde_json::Value {
    // Gename field "title"
    if let serde_json::Value::Object(mut object) = schema {
        object.insert("title".to_string(), serde_json::Value::String(title.to_string()));
        serde_json::Value::Object(object)
    } else {
        schema
    }
}

fn generate_and_store_schema<T: JsonSchema>(title: &str, output_path: &str) -> Result<(), anyhow::Error> {
    let schema = schema_for!(T);
    let schema = rename_schema(schema.to_value(), title);
    let schema_json = serde_json::to_string_pretty(&schema)?;
    std::fs::write(output_path, schema_json)?;
    Ok(())
}
